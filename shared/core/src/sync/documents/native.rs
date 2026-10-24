//! Composition with the authoritative account and private physical agent ports.
use super::{DocumentsHost, Identity, Port};
use crate::{
    agent::{storage::AgentStore, AgentClient, Cancellation, Capability, Request},
    features::{
        billing::host::Opener,
        dictionary::Events,
        styles::{
            view::{ApplicationChoice, Facts},
            Effect,
        },
        sync::{Diagnostics, Scope},
        sync_documents::AccountTransport,
    },
};
use govorun_core::AccountSession;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeviceFacts {
    permissions_ready: bool,
    #[serde(default)]
    network_available: Option<bool>,
    paused: bool,
    self_hosted: bool,
    self_hosted_ready: bool,
    self_hosted_enabled: bool,
    self_hosted_title: String,
    applications: Vec<ApplicationChoice>,
}
#[path = "stylization.rs"]
mod stylization;
pub struct NativeDocuments {
    stylization: std::sync::OnceLock<Arc<crate::features::stylization::Stylization>>,
    style_metadata: std::sync::Mutex<Option<stylization::Metadata>>,
    self_host: std::sync::OnceLock<Arc<crate::features::self_host::SelfHost>>,
    recording_readiness: std::sync::OnceLock<Arc<dyn Fn() -> bool + Send + Sync>>,
    core: AccountSession,
    agent: AgentClient,
    open: Opener,
    diagnostic_tasks: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
}
impl NativeDocuments {
    pub fn new(core: AccountSession, agent: AgentClient, open: Opener) -> Arc<Self> {
        Arc::new(Self {
            stylization: std::sync::OnceLock::new(),
            style_metadata: std::sync::Mutex::new(None),
            self_host: std::sync::OnceLock::new(),
            recording_readiness: std::sync::OnceLock::new(),
            core,
            agent,
            open,
            diagnostic_tasks: std::sync::Mutex::new(vec![]),
        })
    }
    pub fn attach_recording_readiness(&self, read: Arc<dyn Fn() -> bool + Send + Sync>) {
        let _ = self.recording_readiness.set(read);
    }
    pub fn attach_self_host(&self, owner: Arc<crate::features::self_host::SelfHost>) {
        let _ = self.self_host.set(owner);
    }
    pub fn diagnostics(self: &Arc<Self>) -> Diagnostics {
        let owner = self.clone();
        Arc::new(move |event, failure| {
            let reason = Some(failure.code)
                .filter(|code| crate::journal_report::input::code(code) && failure.status.is_none());
            owner.queue_diagnostic(match reason {
                Some(reason) => json!({"event":event,"status":null,"reason":reason}),
                None => json!({"event":event,"status":failure.status}),
            })
        })
    }
    pub async fn recording_capture_ready(&self, detail: String) {
        let _ = self.call("data.diagnostic", json!({"event":"audio_capture_ready","detail":detail})).await;
    }
    pub fn recording_completed(self: &Arc<Self>) {
        self.queue_diagnostic(json!({"event":"asr_completed","detail":""}));
    }
    pub fn recording_timing(self: &Arc<Self>, timing: String) {
        self.queue_diagnostic(json!({"event":"stylization_timing","timing":timing}));
    }
    pub fn self_host_failed(self: &Arc<Self>, status: Option<u16>) {
        self.queue_diagnostic(json!({"event":"self_host_request_failed","status":status.filter(|value| (100..=599).contains(value))}));
    }
    fn queue_diagnostic(self: &Arc<Self>, value: Value) {
        let owner = self.clone();
        let mut tasks = self.diagnostic_tasks.lock().unwrap();
        tasks.retain(|task| !task.is_finished());
        tasks.push(tokio::spawn(async move {
            let _ = owner.call("data.diagnostic", value).await;
        }));
    }
    pub async fn drain_diagnostics(&self) {
        let tasks = std::mem::take(&mut *self.diagnostic_tasks.lock().unwrap());
        for task in tasks {
            let _ = task.await;
        }
    }
    pub fn build(
        self: &Arc<Self>,
        events: Events,
        diagnostics: Diagnostics,
    ) -> Result<Arc<DocumentsHost>, &'static str> {
        let owner = self.clone();
        let identity = Arc::new(move || owner.identity());
        let owner = self.clone();
        let port: Port = Arc::new(move |kind, value| {
            let owner = owner.clone();
            Box::pin(async move {
                // The document is already durable. Revoke opted-out work before
                // a cache ACK can suspend; native mirrors cannot grant authority.
                if kind == "styles" {
                    owner.configure_stylization(&value).await?;
                }
                if owner.agent.supports(&Capability::named("data.configure")?) {
                    // Explicit synthetic/old physical fixtures still verify ACK failures.
                    owner
                        .call("data.configure", json!({"kind":kind,"value":value}))
                        .await?;
                }
                Ok(())
            })
        });
        let owner = self.clone();
        let facts = Arc::new(move || {
            let owner = owner.clone();
            Box::pin(async move { owner.style_facts().await })
                as std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<Facts, &'static str>> + Send>,
                >
        });
        let host = DocumentsHost::new(
            Arc::new(AgentStore::new(self.agent.clone())),
            Arc::new(AccountTransport(self.core.clone())),
            identity,
            port,
            facts,
            events,
            diagnostics,
            Arc::new(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            }),
            Arc::new(|| chrono::Local::now().format("%Y-%m-%d").to_string()),
        );
        let owner = self.clone();
        host.attach_style_effects(Arc::new(move |effect| {
            let owner = owner.clone();
            Box::pin(async move { owner.effect(effect).await })
        }))?;
        Ok(host)
    }
    pub fn identity(&self) -> Identity {
        let before = self.core.snapshot();
        let profile = self.core.profile();
        let after = self.core.snapshot();
        let scope = if before.revision == after.revision && after.signed_in {
            profile.map(|p| Scope {
                account: p.id().to_owned(),
                generation: after.session_generation,
            })
        } else {
            None
        };
        Identity {
            scope,
            signed_in: after.signed_in,
        }
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        self.agent
            .call(
                Request {
                    capability: Capability::named(method)?,
                    params,
                },
                Duration::from_secs(20),
                &Cancellation::default(),
            )
            .await
    }
    pub async fn style_facts(&self) -> Result<Facts, &'static str> {
        let captured = self.core.snapshot();
        let mut device: DeviceFacts =
            serde_json::from_value(self.call("data.localFacts", json!({})).await?)
                .map_err(|_| "styles_invalid_response")?;
        if let Some(read) = self.recording_readiness.get() {
            device.permissions_ready = read();
        }
        if let Some(owner) = self.self_host.get() {
            let snapshot = owner.snapshot().await?;
            device.self_hosted = snapshot["routeEnabled"] == true;
            device.self_hosted_ready = snapshot["busy"] == false
                && snapshot["configured"] == true
                && snapshot["consent"] == true
                && snapshot["verified"] == true;
            device.self_hosted_enabled = snapshot["stylingEnabled"] == true;
            device.self_hosted_title = if device.self_hosted_enabled {
                "Общий"
            } else {
                "Без стилизации"
            }
            .into();
        }
        if device.self_hosted_title.encode_utf16().count() > 512 || device.applications.len() > 4096
        {
            return Err("styles_invalid_response");
        }
        let mut ids = std::collections::HashSet::new();
        if device.applications.iter().any(|v| {
            v.id.trim().is_empty()
                || v.name.trim().is_empty()
                || v.id.encode_utf16().count() > 512
                || v.name.encode_utf16().count() > 512
                || v.id.chars().any(char::is_control)
                || v.name.chars().any(char::is_control)
                || !ids.insert(&v.id)
        }) {
            return Err("styles_invalid_response");
        }
        let owner = self.stylization.get().ok_or("stylization_unavailable")?;
        device.paused = owner.paused().map_err(|error| error.code())?;
        let observation = owner.observation(false, false).await.map_err(|error| error.code())?;
        let storage_available = captured.storage_available && observation.storage_available();
        let window = observation.window;
        if self.core.snapshot().revision != captured.revision {
            return Err("styles_snapshot_changed");
        }
        Ok(Facts {
            access: json!({"generation":captured.session_generation,"signedIn":captured.signed_in,
            "profileFresh":captured.profile_fresh,"storageAvailable":storage_available,"permissionsReady":device.permissions_ready,
            "termsMatch":captured.access.terms_match,"entitled":captured.access.entitlement_active,"authorized":captured.access.authorized,
            "consentAccepted":captured.access.consent_accepted,"processingReady":captured.access.processing_ready,
            "starterLocallyExhausted":window["starterLocallyExhausted"],"availability":window["availability"],
            "processingReadiness":window["processingReadiness"],"selfHosted":device.self_hosted,"selfHostedReady":device.self_hosted_ready,
            "clientTermsVersion":captured.access.client_terms_version,"localTermsVersion":0,
            "enabled":device.self_hosted_enabled,"paused":device.paused,"presetCount":0}),
            self_hosted_title: device.self_hosted_title,
            applications: device.applications,
        })
    }
    fn current(&self, generation: u64) -> Result<(), &'static str> {
        if self.core.snapshot().session_generation == generation {
            Ok(())
        } else {
            Err("account_session_changed")
        }
    }
    pub async fn effect(&self, effect: Effect) -> Result<Value, &'static str> {
        match effect {
            Effect::AcceptTerms {
                generation,
                version,
            } => {
                self.core
                    .accept_online_terms(generation, version, true)
                    .await
                    .map_err(|e| e.code())?;
                Ok(Value::Null)
            }
            Effect::ConsentTimestamp { generation } => {
                self.current(generation)?;
                let snapshot = self.core.snapshot();
                if !snapshot.profile_fresh || !snapshot.access.consent_accepted {
                    return Err("account_consent_unconfirmed");
                }
                let profile = self.core.profile().ok_or("account_signed_out")?;
                let value = &profile.wire()["onlineConsent"]["acceptedAt"];
                let millis = if value.is_null() {
                    None
                } else {
                    let raw = value.as_str().ok_or("account_invalid_response")?;
                    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
                        .map_err(|_| "account_invalid_response")?
                        .timestamp_millis();
                    Some(u64::try_from(parsed).map_err(|_| "account_invalid_response")?)
                };
                if self.core.snapshot().revision != snapshot.revision {
                    return Err("account_session_changed");
                }
                Ok(json!(millis))
            }
            Effect::Refresh { generation } => {
                self.current(generation)?;
                self.core
                    .refresh_for_activity(Duration::ZERO)
                    .await
                    .map_err(|e| e.code())?;
                self.current(generation)?;
                self.prepare_stylization(generation).await
            }
            Effect::PreparePasses { generation } => {
                self.current(generation)?;
                self.prepare_stylization(generation).await
            }
            Effect::SelfHosted { enabled } => {
                self.self_host
                    .get()
                    .ok_or("self_host_unavailable")?
                    .set_styling(enabled)
                    .await?;
                Ok(Value::Null)
            }
            Effect::Documents => {
                (self.open)("https://govorun.club/legal")?;
                Ok(Value::Null)
            }
            Effect::LoadApplications => {
                self.call("data.styleEffect", json!({"effect":"loadApplications"}))
                    .await
            }
        }
    }
}
