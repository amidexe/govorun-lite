use crate::{profile::Profile, model_store, windows};
use govorun_core::update_policy::{self, Channel, CheckReason, Preferences};
use std::{io::Read, path::PathBuf, sync::{Mutex, Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};
use govorun_core::boundary_snapshots::Release;
#[path = "update_actions.rs"]
mod actions;

pub use govorun_core::boundary_snapshots::{UpdateSnapshot as Snapshot, UpdateTransfer as Transfer};
struct Value {
    preferences: Preferences, status: &'static str, release: Option<Release>, available: Option<Update>,
    candidate: Option<crate::update_install::Candidate>, cancel: Option<Arc<crate::update_cancel::Cancellation>>,
    transfer: Option<Transfer>, error: Option<&'static str>,
    channel_selectable: bool,
}
pub struct Updates { path: PathBuf, feed: Option<url::Url>, value: Mutex<Value>, operation: AtomicBool }

impl Updates {
    pub fn load(app: &AppHandle) -> Result<Self, &'static str> {
        let profile = Profile::from_identifier(&app.config().identifier).ok();
        let path = app.path().app_config_dir().map_err(|_| "updates_unavailable")?.join("updates.json");
        let mut preferences = std::fs::File::open(&path).ok().and_then(|file| {
            let mut bytes = Vec::new(); file.take((update_policy::MAX_PREFERENCES_BYTES + 1) as u64).read_to_end(&mut bytes).ok()?;
            (bytes.len() <= update_policy::MAX_PREFERENCES_BYTES).then(|| serde_json::from_slice::<Preferences>(&bytes).ok()).flatten()
        }).unwrap_or_default();
        let feed = option_env!("GOVORUN_UPDATE_FEED_URL").filter(|url| !url.is_empty() && profile.is_some())
            .and_then(|url| update_policy::official_url(url).ok());
        let channel_selectable = matches!(profile, Some(Profile::Ordinary)) && feed.is_some();
        if !channel_selectable { preferences.channel = Some(Channel::Test); }
        else if preferences.channel.is_none() {
            let channel = if feed.as_ref().is_some_and(|url| url.as_str() == Channel::Stable.feed()) {
                Channel::Stable
            } else { Channel::Test };
            // Old testers retain their data and enter the public test channel.
            // A cached legacy manifest must not appear in another channel.
            update_policy::Preferences::select_channel(&mut preferences, channel);
        }
        let status = if feed.is_none() { "disabled" } else { "idle" };
        Ok(Self { path, feed, value: Mutex::new(Value { preferences, status, release: None, available: None,
            candidate: None, cancel: None, transfer: None, error: None, channel_selectable }), operation: AtomicBool::new(false) })
    }

    fn snapshot(value: &Value) -> Snapshot {
        Snapshot { status: value.status, release: value.release.clone(), dismissed: value.release.as_ref()
            .is_some_and(|release| value.preferences.dismissed.as_deref() == Some(&release.version)), transfer: value.transfer.clone(), error: value.error,
            channel: value.preferences.channel.unwrap_or_default(), channel_selectable: value.channel_selectable }
    }

    fn source(&self, value: &Value) -> Result<url::Url, &'static str> {
        let channel = value.preferences.channel.unwrap_or_default();
        let feed = if value.channel_selectable { update_policy::official_url(channel.feed())? }
            else { self.feed.clone().ok_or("updates_unavailable")? };
        Ok(feed)
    }

    fn select_channel(&self, channel: Channel) -> Result<Snapshot, &'static str> {
        let _active = self.enter()?;
        let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
        if !value.channel_selectable { return Err("updates_channel_unavailable"); }
        if matches!(value.status, "checking" | "downloading" | "installing") { return Err("updates_busy"); }
        if value.preferences.channel == Some(channel) { return Ok(Self::snapshot(&value)); }
        let mut preferences = value.preferences.clone();
        update_policy::Preferences::select_channel(&mut preferences, channel);
        self.save(&preferences)?;
        value.preferences = preferences;
        value.release = None; value.available = None; value.candidate = None; value.transfer = None; value.error = None; value.status = "idle";
        Ok(Self::snapshot(&value))
    }

    fn save(&self, preferences: &Preferences) -> Result<(), &'static str> {
        model_store::plain_path(&self.path)?;
        let directory = self.path.parent().ok_or("updates_unavailable")?;
        std::fs::create_dir_all(directory).map_err(|_| "updates_save_failed")?;
        // Only release metadata and local update preferences, never account data.
        let bytes = serde_json::to_vec(preferences).map_err(|_| "updates_save_failed")?;
        let pending = self.path.with_extension("pending");
        model_store::plain_path(&pending)?;
        std::fs::write(&pending, bytes).and_then(|_| std::fs::rename(&pending, &self.path)).map_err(|_| "updates_save_failed")
    }

    async fn check(&self, app: &AppHandle, reason: CheckReason) -> Result<Snapshot, &'static str> {
        let account = app.state::<crate::service::Services>().account_session().await;
        let signed_in = account.as_ref().is_some_and(|owner| {
            let state = owner.snapshot(); state.signed_in && state.storage_available && !state.logout_pending
        });
        let mut boundary = account.as_ref().map(|owner| owner.subscribe_boundary());
        if self.feed.is_none() || !CheckReason::allowed(reason, signed_in) {
            return self.value.lock().map(|value| Self::snapshot(&value)).map_err(|_| "updates_unavailable");
        }
        let feed = {
            let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "updates_unavailable")?.as_secs();
            if matches!(value.status, "checking" | "downloading" | "ready" | "installing")
                || self.operation.load(Ordering::SeqCst)
                || !value.preferences.due(now, reason) { return Ok(Self::snapshot(&value)); }
            value.preferences.last_attempt = now;
            // A storage error must not turn each focus event into another network request.
            let _ = self.save(&value.preferences);
            value.status = "checking";
            self.source(&value)?
        };
        let result = tokio::select! {
            biased;
            _ = async {
                match &mut boundary {
                    Some(changes) => { let _ = changes.changed().await; },
                    None => std::future::pending().await,
                }
            } => Err("updates_cancelled"),
            result = check_update(app, feed) => result,
        };
        let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
        match result {
            Ok(update) => {
                value.status = if update.is_some() { "available" } else { "current" };
                value.release = update.as_ref().map(|update| Release { version: update.version.clone(), notes: update.body.clone().unwrap_or_default() });
                value.available = update;
                let _ = self.save(&value.preferences);
            }
            Err(_) => value.status = "unavailable",
        }
        Ok(Self::snapshot(&value))
    }
}

async fn check_update(app: &AppHandle, feed: url::Url) -> Result<Option<Update>, &'static str> {
    let builder = app.updater_builder().endpoints(vec![feed]).map_err(|_| "updates_invalid_feed")?
        .timeout(Duration::from_secs(8))
        .configure_client(|client| client.https_only(true).redirect(reqwest::redirect::Policy::none()));
    // Services have already drained before installation. Keep windows alive
    // until ShellExecute succeeds so a launch failure can resume the app.
    #[cfg(windows)]
    let builder = builder.on_before_exit(|| {});
    let updater = builder.build().map_err(|_| "updates_unavailable")?;
    let mut update = updater.check().await.map_err(|_| "updates_unavailable")?;
    if let Some(update) = &mut update {
        update_policy::official_url(update.download_url.as_str())?;
        if update.version.len() > 64 || update.body.as_ref().is_some_and(|body| body.chars().count() > 1500) {
            return Err("updates_invalid_feed");
        }
        // Download includes the model. Keep the short check timeout separate
        // from the package's transfer budget; Tauri verifies minisign itself.
        update.timeout = Some(Duration::from_secs(600));
    }
    Ok(update)
}

async fn command(app: AppHandle, window: WebviewWindow, state: State<'_, Updates>, action: Option<String>, version: Option<String>, manual: Option<bool>, channel: Option<Channel>) -> Result<Snapshot, &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    let action = action.as_deref().unwrap_or("snapshot");
    if action == "channel" { return state.select_channel(channel.ok_or("updates_invalid_channel")?); }
    if action == "check" {
        let reason = if manual.unwrap_or(false) { CheckReason::Manual } else { CheckReason::Automatic };
        return state.check(&app, reason).await;
    }
    if action == "download" { return state.start_download(&app, version.as_deref()).await; }
    if action == "install" { return state.install_candidate(&app, version.as_deref()).await; }
    if action == "cancel" { return state.cancel_download(&app); }
    if !matches!(action, "snapshot" | "dismiss") { return Err("updates_invalid_action"); }
    let mut value = state.value.lock().map_err(|_| "updates_unavailable")?;
    if action == "dismiss" {
        let release = value.release.as_ref().filter(|release| Some(release.version.as_str()) == version.as_deref()).ok_or("updates_stale_release")?;
        let preferences = Preferences { dismissed: Some(release.version.clone()), last_attempt: value.preferences.last_attempt,
            channel: value.preferences.channel };
        state.save(&preferences)?; value.preferences = preferences;
    }
    Ok(Updates::snapshot(&value))
}

#[tauri::command]
pub async fn updates(app: AppHandle, window: WebviewWindow, state: State<'_, Updates>, action: Option<String>, version: Option<String>, manual: Option<bool>, channel: Option<Channel>) -> Result<Snapshot, &'static str> {
    let result = command(app.clone(), window, state, action, version, manual, channel).await;
    if let Ok(snapshot) = &result {
        crate::attention::event(&app, "updates.changed", &serde_json::to_value(snapshot).unwrap_or_default());
    }
    result
}
