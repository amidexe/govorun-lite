//! One complete style-screen view; authority stays in the common core.
mod actions;
mod activation;
mod subscription;
mod snapshot;
pub use activation::{Effect, Effects, Io};
pub mod view;
use super::{dictionary::Events, snapshot_gate::SnapshotGate, style_library::StyleLibraryFeature};
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex as StateMutex, OnceLock},
};
use tokio::sync::Mutex;
pub type Facts = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<view::Facts, &'static str>> + Send>>
        + Send
        + Sync,
>;
pub struct StylesFeature {
    library: Arc<StyleLibraryFeature>,
    facts: Facts,
    scope: Arc<dyn Fn(u64) -> String + Send + Sync>,
    events: Events,
    gate: SnapshotGate,
    serial: Mutex<()>,
    state: StateMutex<activation::State>,
    effects: OnceLock<Effects>,
    tasks: StateMutex<Vec<tokio::task::JoinHandle<()>>>,
    last_snapshot: StateMutex<Option<Value>>,
}
impl StylesFeature {
    pub fn new(
        library: Arc<StyleLibraryFeature>,
        facts: Facts,
        scope: Arc<dyn Fn(u64) -> String + Send + Sync>,
        events: Events,
    ) -> Arc<Self> {
        Arc::new(Self {
            library,
            facts,
            scope,
            events,
            gate: SnapshotGate::default(),
            serial: Mutex::new(()),
            state: StateMutex::new(activation::State::default()),
            effects: OnceLock::new(),
            tasks: StateMutex::new(Vec::new()),
            last_snapshot: StateMutex::new(None),
        })
    }
    pub async fn snapshot(&self) -> Result<Value, &'static str> {
        let _serial = self.serial.lock().await;
        self.read_snapshot().await
    }
    async fn read_snapshot_once(&self) -> Result<Value, &'static str> {
        if self.state.lock().unwrap().stopped {
            return Err("styles_stopped");
        }
        let mut facts = (self.facts)().await?;
        let mut library = self
            .library
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        // Native facts can change during a storage/cache ACK. Do not publish a
        // ready card composed with the account or service state from before it.
        let current = (self.facts)().await?;
        if facts != current {
            return Err("styles_snapshot_changed");
        }
        // Device preference mirrors come from the same document owner as edits,
        // not from a possibly stale native cache. Self-host preferences remain
        // owned by the settings feature.
        let metadata = &library["configuration"]["metadata"];
        facts.access["localTermsVersion"] = metadata["localTermsVersion"].clone();
        if facts.access["selfHosted"] != true {
            facts.access["enabled"] = metadata["enabled"].clone();
        }
        let generation = facts.access["generation"]
            .as_u64()
            .ok_or("styles_invalid_response")?;
        let presentation =
            activation::core(json!({"operation":"styleAccessPresentation","facts":facts.access}))?;
        if presentation["restoreConsent"] == true {
            if let Some(effects) = self.effects.get() {
                let timestamp = (effects.io)(Effect::ConsentTimestamp { generation }).await?;
                let current = (self.facts)().await?;
                if current.access["generation"] != generation || self.state.lock().unwrap().stopped
                {
                    return Err("styles_session_changed");
                }
                let revision = library["configuration"]["revision"]
                    .as_str()
                    .ok_or("styles_invalid_response")?
                    .to_owned();
                library = self
                    .library
                    .execute(
                        json!({"kind":"recordConsent", "version":facts.access["clientTermsVersion"],
                    "acceptedAtMillis":timestamp}),
                        Some(revision),
                        false,
                    )
                    .await?;
                let refreshed = (self.facts)().await?;
                if refreshed.access["generation"] != generation {
                    return Err("styles_session_changed");
                }
                // Recompose after the cache ACK; restoration grants no access.
                facts = refreshed;
                facts.access["localTermsVersion"] =
                    library["configuration"]["metadata"]["localTermsVersion"].clone();
                if facts.access["selfHosted"] != true {
                    facts.access["enabled"] =
                        library["configuration"]["metadata"]["enabled"].clone();
                }
            }
        }
        let (busy, message) = self.presentation(generation)?;
        let view = view::compose(library, facts, (self.scope)(generation), busy, message)?;
        serde_json::to_value(view).map_err(|_| "styles_invalid_response")
    }
    pub async fn publish(&self) -> Result<(), &'static str> {
        let view = self.snapshot().await?;
        if self.gate.changed(&view) {
            (self.events)("styles.changed", view);
        }
        Ok(())
    }
}
