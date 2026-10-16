//! Same-process setup effects. Core owns settings and validation; this port
//! observes OS state and applies only the already accepted physical command.
use crate::features::permission_setup::settings::{Owner, Ports};
use crate::{
    account_host::AccountHost,
    recording_runtime::{Runtime, Settings},
};
use govorun_core::{
    permission_setup::{
        self, Acceleration, Command, Device, Field, Login, Marker, Microphone, Permission,
        Permissions, Snapshot,
    },
    DocumentStore, Future,
};
use govorun_os_runtime::setup::{self, access, login};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use tauri::AppHandle;
pub struct Physical {
    app: AppHandle,
    runtime: OnceLock<Weak<Runtime>>,
    account: Arc<AccountHost>,
    store: Arc<dyn DocumentStore>,
    owner: OnceLock<Weak<Owner>>,
    executable: PathBuf,
    identity: String,
    observed: Mutex<Option<Snapshot>>,
}
async fn main<T: Send + 'static>(
    app: &AppHandle,
    action: impl FnOnce() -> Result<T, &'static str> + Send + 'static,
) -> Result<T, &'static str> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = send.send(action());
    })
    .map_err(|_| "dictation_setup_unavailable")?;
    receive.await.map_err(|_| "dictation_setup_unavailable")?
}
impl Physical {
    pub fn new(
        app: &AppHandle,
        account: Arc<AccountHost>,
        store: Arc<dyn DocumentStore>,
    ) -> Result<Arc<Self>, &'static str> {
        Ok(Arc::new(Self {
            app: app.clone(),
            runtime: OnceLock::new(),
            account,
            store,
            owner: OnceLock::new(),
            executable: std::env::current_exe().map_err(|_| "login_unavailable")?,
            identity: app.config().identifier.clone(),
            observed: Mutex::new(None),
        }))
    }
    pub fn attach_runtime(&self, runtime: &Arc<Runtime>) {
        let _ = self.runtime.set(Arc::downgrade(runtime));
    }
    fn runtime(&self) -> Result<Arc<Runtime>, &'static str> {
        self.runtime
            .get()
            .and_then(Weak::upgrade)
            .ok_or("dictation_stopped")
    }
    pub fn attach(&self, owner: &Arc<Owner>) {
        let _ = self.owner.set(Arc::downgrade(owner));
    }
    fn refreshed(&self) -> Arc<dyn Fn() + Send + Sync> {
        let owner = self.owner.get().cloned().unwrap_or_default();
        Arc::new(move || {
            let owner = owner.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(owner) = owner.upgrade() {
                    let _ = owner.refresh_execution().await;
                }
            });
        })
    }
    pub async fn keep_clipboard(&self) -> Result<bool, &'static str> {
        #[cfg(target_os = "macos")]
        {
            let value = govorun_core::clipboard_settings::ClipboardSettings::new(
                self.store.clone(),
            )
            .invoke("clipboard.snapshot", json!({}))
            .await?;
            Ok(value["clipboard"] == "copy")
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(false)
        }
    }
    pub fn cached(&self) -> Result<Snapshot, &'static str> {
        self.observed
            .lock()
            .map_err(|_| "dictation_setup_unavailable")?
            .clone()
            .ok_or("dictation_setup_unavailable")
    }
    pub fn microphone_ready(&self) -> bool {
        self.cached().is_ok_and(|setup| {
            setup
                .permissions
                .microphone
                .can_record(std::env::consts::OS)
        })
    }
    pub async fn observed(&self) -> Result<Snapshot, &'static str> {
        let document = self
            .store
            .read_document(permission_setup::settings::KEY.into())
            .await
            .map_err(|_| "dictation_storage_unavailable")?
            .ok_or("dictation_storage_unavailable")?;
        let values = permission_setup::settings::values(&document.value)?;
        let key = if cfg!(target_os = "macos") {
            "audioInputDeviceUID"
        } else {
            "audioInputDevice"
        };
        let preferred = values
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let catalog = tokio::task::spawn_blocking(move || setup::catalog(&preferred))
            .await
            .map_err(|_| "microphone_enumeration_failed")??;
        if let Ok(runtime) = self.runtime() {
            tokio::task::spawn_blocking(move || runtime.refresh_hook())
                .await
                .map_err(|_| "hotkeys_unavailable")??;
        }
        let identity = self.identity.clone();
        let executable = self.executable.clone();
        let (microphone, launch_at_login) = main(&self.app, move || {
            #[cfg(target_os = "macos")]
            let login = {
                let _ = (identity, executable);
                setup::login_status()
            };
            #[cfg(target_os = "windows")]
            let login = setup::login_status(&identity, &executable);
            Ok((setup::microphone(), login))
        })
        .await?;
        let microphone = match microphone {
            access::Microphone::Authorized => Microphone::Authorized,
            access::Microphone::NotDetermined => Microphone::NotDetermined,
            access::Microphone::Denied => Microphone::Denied,
            access::Microphone::Restricted => Microphone::Restricted,
            access::Microphone::Unknown => Microphone::Unknown,
        };
        let launch_at_login = match launch_at_login {
            login::Login::Enabled => Login::Enabled,
            login::Login::Disabled => Login::Disabled,
            login::Login::ApprovalRequired => Login::ApprovalRequired,
            login::Login::Unavailable => Login::Unavailable,
        };
        let windows = cfg!(target_os = "windows");
        let constants = crate::features::dictation::constants()
            .map_err(|_| "dictation_invalid_facts")?;
        let snapshot = Snapshot {
            microphones: catalog
                .inputs
                .into_iter()
                .map(|input| Device {
                    id: input.id,
                    name: input.name,
                })
                .collect(),
            microphone_id: None,
            launch_at_login,
            play_sounds: true,
            mute_audio: false,
            max_minutes: 5,
            duration_mode: self.account.recording_setup_mode().await?,
            acceleration: Acceleration::Automatic,
            voice_marker: Marker::Off,
            marker_minimum: constants["minimumCharacters"]
                .as_u64()
                .ok_or("dictation_invalid_facts")?,
            permissions: Permissions {
                microphone,
                accessibility: self.runtime().map_or_else(
                    |_| govorun_os_runtime::keyboard::permission_ready(),
                    |runtime| runtime.hook_ready(),
                ),
            },
            push_to_talk: windows.then_some(false),
            keep_model_loaded: windows.then_some(true),
            always_on_microphone: windows.then_some(false),
            input_channel: windows.then_some(Value::Null),
            input_channels: windows.then_some(catalog.channels),
        };
        let snapshot = permission_setup::settings::overlay(&document.value, snapshot)?;
        *self
            .observed
            .lock()
            .map_err(|_| "dictation_setup_unavailable")? = Some(snapshot.clone());
        Ok(snapshot)
    }
}
impl Ports for Physical {
    fn facts(&self) -> Future<'_, Result<Snapshot, &'static str>> {
        Box::pin(self.observed())
    }
    fn busy(&self) -> bool {
        self.runtime().is_ok_and(|runtime| runtime.busy())
    }
    fn configure<'a>(
        &'a self,
        _: &'a Field,
        _: &'a Snapshot,
        next: &'a Snapshot,
    ) -> Future<'a, Result<(), &'static str>> {
        Box::pin(async move {
            self.runtime()?
                .configure(Settings::from_snapshot(next, self.keep_clipboard().await?)?)
                .await
        })
    }
    fn execute(&self, command: Command) -> Future<'_, Result<(), &'static str>> {
        Box::pin(async move {
            match command {
                Command::Prepare {} => self.runtime()?.prepare().await,
                Command::Set {
                    field: Field::LaunchAtLogin,
                    value,
                } => {
                    let enabled = value.as_bool().ok_or("dictation_invalid_setting")?;
                    let identity = self.identity.clone();
                    let executable = self.executable.clone();
                    main(&self.app, move || {
                        #[cfg(target_os = "macos")]
                        {
                            let _ = (identity, executable);
                            setup::set_login(enabled)
                        }
                        #[cfg(target_os = "windows")]
                        {
                            setup::set_login(&identity, &executable, enabled)
                        }
                    })
                    .await
                }
                Command::LoginItems {} => {
                    main(&self.app, move || {
                        #[cfg(target_os = "macos")]
                        {
                            setup::open_login_settings()
                        }
                        #[cfg(target_os = "windows")]
                        {
                            crate::features::about::open::open("ms-settings:startupapps")
                        }
                    })
                    .await
                }
                Command::Permission { permission } => {
                    let refreshed = self.refreshed();
                    main(&self.app,move || {
                        #[cfg(target_os="macos")]
                        { let url=match permission {
                            Permission::Microphone => {
                                if setup::request_microphone(refreshed)? {return Ok(());}
                                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
                            },
                            Permission::Accessibility => if govorun_os_runtime::keyboard::permission_ready() {
                                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
                            } else { "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility" }
                        };crate::features::about::open::open(url) }
                        #[cfg(target_os="windows")]
                        {let _=refreshed;match permission {Permission::Microphone=>crate::features::about::open::open("ms-settings:privacy-microphone"),Permission::Accessibility=>Err("invalid_command")}}
                    }).await
                }
                _ => Err("invalid_command"),
            }
        })
    }
}
