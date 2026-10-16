//! Composition only: Android uses the desktop Stylization owner and supplies
//! physical storage, transport, settings and connectivity through JNI effects.
use crate::{session_host::SessionHost, session_style_ports::StylePorts};
use govorun_core::{
    features::stylization::{commands::Command, ipc::{Deliver, Guard}, Stylization},
    style_pass_vault::{format::Format, Repository},
    stylization::{reporting::Settings, scheduling::Trigger},
    AccountSession, Error, Result,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

pub(crate) struct StylesBridge {
    pub owner: Arc<Stylization>,
    account: AccountSession,
    host: Arc<SessionHost>,
    publications: Mutex<Vec<JoinHandle<()>>>,
    projection: AsyncMutex<Option<Value>>,
}
impl StylesBridge {
    pub async fn create(account: AccountSession, host: Arc<SessionHost>) -> Result<Arc<Self>> {
        let ports = Arc::new(StylePorts(host.clone()));
        let repository = Arc::new(Repository::new(Arc::new(govorun_core::style_pass_vault::SessionStore::new(ports.clone(),Format::AndroidV8)),Format::AndroidV8));
        let mut changes = repository.subscribe_custody();
        let owner = Stylization::new(account.clone(),
            repository, Self::gateway(ports)?,
            Arc::new(||std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_millis() as u64));
        let bridge = Arc::new(Self {owner,account,host,publications:Mutex::new(Vec::new()),
            projection:AsyncMutex::new(None)});
        let (_,initially_online) = bridge.facts().await?;
        let weak = Arc::downgrade(&bridge);
        let enabled = Arc::new(move || {
            let weak = weak.clone();
            Box::pin(async move {
                let bridge = weak.upgrade().ok_or(Error::SessionChanged)?;
                let (selected,online) = bridge.facts().await?;
                Ok(selected && online)
            }) as govorun_core::Future<'static,Result<bool>>
        });
        let weak = Arc::downgrade(&bridge);
        let online = Arc::new(move || {
            let weak = weak.clone();
            Box::pin(async move {
                match weak.upgrade() {Some(bridge)=>bridge.facts().await.is_ok_and(|(_,online)|online),None=>false}
            }) as govorun_core::Future<'static,bool>
        });
        let weak = Arc::downgrade(&bridge);
        let changed = Arc::new(move |value| {
            if let Some(bridge) = weak.upgrade() {
                bridge.queue_effect(json!({"operation":"stylesAvailability","value":value}));
            }
        });
        bridge.owner.start_background(enabled,online,initially_online,changed)?;
        bridge.owner.set_timing_sink(Arc::new(crate::session_timing::record))?;
        let weak = Arc::downgrade(&bridge);
        bridge.publications.lock().expect("style publications").push(tokio::spawn(async move {
            while changes.changed().await.is_ok() {
                let Some(bridge) = weak.upgrade() else {break;};
                let _ = bridge.publish().await;
            }
        }));
        Ok(bridge)
    }
    fn gateway(ports: Arc<StylePorts>) -> Result<Arc<dyn govorun_core::stylization::Gateway>> {
        #[cfg(feature = "private-fixtures")]
        { Ok(ports) }
        #[cfg(not(feature = "private-fixtures"))]
        {
            let _ = ports;
            Ok(Arc::new(govorun_styles_http::Client::new(
                govorun_core::stylization::configured_origin(), "Govorun-Android/2")?
                .with_version(option_env!("GOVORUN_APP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")))?))
        }
    }
    fn queue_effect(self: &Arc<Self>, value: Value) {
        let mut tasks = self.publications.lock().expect("style publications");
        tasks.retain(|task|!task.is_finished());
        let host = self.host.clone();
        tasks.push(tokio::spawn(async move {let _ = host.effect(value,false).await;}));
    }
    async fn facts(&self) -> Result<(bool,bool)> {
        let value = self.host.effect(json!({"operation":"styleFacts"}),false).await?;
        let settings = Settings {
            official_enabled:value["enabled"].as_bool().ok_or(Error::InvalidResponse)?,
            self_hosted:value["selfHosted"].as_bool().ok_or(Error::InvalidResponse)?,
            accepted_terms:value["acceptedTerms"].as_u64().and_then(|v|u32::try_from(v).ok()).ok_or(Error::InvalidResponse)?,
        };
        self.owner.configure_reporting(Some(settings))?;
        let online = value["networkAvailable"].as_bool().ok_or(Error::InvalidResponse)?;
        Ok((settings.official_selected(self.account.snapshot().access.client_terms_version),online))
    }
    pub async fn dispatch(&self, command: Value, guard: &Guard, id: u64) -> Result<Value> {
        if command["operation"] == "stylizationStart" {
            self.facts().await?;
            self.owner.maintain(self.account.snapshot().session_generation,Trigger::Maintenance,true)?;
            self.publish().await?;
            return Ok(json!({}));
        }
        let command = Command::decode(command)?;
        let is_process = matches!(command, Command::Process { .. });
        let host = self.host.clone();
        let deliver: Deliver = Arc::new(move |value| {
            let host = host.clone();
            Box::pin(async move {host.effect(json!({"operation":"styleProgress","callId":id,"value":value}),false).await.map(|_|())})
        });
        let result = if matches!(command,Command::BeginRecording {..}|Command::Process {..}|Command::Prepare {..}|Command::PreparationPlan {..}|Command::JoinPreparation {..}|Command::SettingsChanged {..}) {
            if let Command::PreparationPlan {session_generation,..} | Command::JoinPreparation {session_generation,..} = &command {self.account.current(*session_generation)?;}
            let (selected,online) = self.facts().await?;
            guard.run_configured(&self.owner,command,deliver,selected,online).await
        } else {guard.run(&self.owner,command,deliver).await};
        // The existing change subscriber publishes processing updates; never
        // delay text delivery behind the UI or turn paid work into a retry.
        if !is_process { let _ = self.publish().await; }
        result
    }

    pub async fn publish(&self) -> Result<()> {
        let mut previous = self.projection.lock().await;
        let epoch = self.account.snapshot().session_generation;
        let observed = self.owner.dispatch(Command::Observation {},&|_|{}).await;
        // A superseded Core read is not a new display state, just as on desktop.
        // Keep the last projection; real storage failures remain visible below.
        let value = match &observed {
            Err(Error::SessionChanged) => return Err(Error::SessionChanged),
            Ok(value) => value.clone(),
            Err(error) => error.wire(),
        };
        let projection = json!({"sessionGeneration":epoch,"value":value});
        if previous.as_ref() != Some(&projection) {
            self.host.effect(json!({"operation":"stylesProject","sessionGeneration":epoch,"value":value}),false).await?;
            *previous = Some(projection);
        }
        observed.map(|_|())
    }
    pub async fn shutdown(&self) {
        self.owner.shutdown().await;
        for task in self.publications.lock().expect("style publications").drain(..) {task.abort();}
    }
}
