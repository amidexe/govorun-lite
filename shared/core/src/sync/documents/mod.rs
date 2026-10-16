//! One admission/lifetime boundary for desktop documents and synchronization.
pub mod native;
mod operations;
use super::{
    dictionary::{Configure, DictionaryFeature, Events},
    statistics::{StatisticsFeature, ViewContext},
    style_library::StyleLibraryFeature,
    styles::{Facts, StylesFeature},
    sync::{Diagnostics, Scope, Stage, SyncDriver},
    sync_documents::{repositories::Repositories, SyncDocuments, Transport},
};
use govorun_core::{
    sync_enrollment::{DataSyncMode, SyncEnrollmentRepository},
    sync_journal::SyncJournalRepository,
    DocumentStore,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub scope: Option<Scope>,
    pub signed_in: bool,
}
pub type IdentityRead = Arc<dyn Fn() -> Identity + Send + Sync>;
pub type Port = Arc<
    dyn Fn(&'static str, Value) -> Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send>>
        + Send
        + Sync,
>;
struct State {
    scope: Option<Scope>,
    mode: DataSyncMode,
    resolved_identity: Option<Identity>,
}
pub struct DocumentsHost {
    local: Arc<Repositories>,
    styles: Arc<StylesFeature>,
    enrollment: SyncEnrollmentRepository,
    journals: Arc<SyncJournalRepository>,
    driver: Arc<SyncDriver>,
    gate: Arc<tokio::sync::Mutex<()>>,
    state: Mutex<State>,
    identity: IdentityRead,
    today: Arc<dyn Fn() -> String + Send + Sync>,
    port: Port,
    pending: AtomicBool,
    stopped: AtomicBool,
}
impl DocumentsHost {
    pub fn new(
        store: Arc<dyn DocumentStore>,
        transport: Arc<dyn Transport>,
        identity: IdentityRead,
        port: Port,
        style_facts: Facts,
        events: Events,
        diagnostics: Diagnostics,
        now: Arc<dyn Fn() -> u64 + Send + Sync>,
        today: Arc<dyn Fn() -> String + Send + Sync>,
    ) -> Arc<Self> {
        let instance = crate::instance::next();
        Arc::new_cyclic(|owner: &Weak<Self>| {
            let dictionary = DictionaryFeature::new(
                store.clone(),
                configure(owner, "dictionary"),
                events.clone(),
            );
            let styles = StyleLibraryFeature::new(store.clone(), configure(owner, "styles"));
            let weak = owner.clone();
            let date = today.clone();
            let statistics = StatisticsFeature::new(
                store.clone(),
                Arc::new(move || {
                    let host = weak.upgrade().ok_or("documents_stopped")?;
                    let identity = (host.identity)();
                    let state = host.state.lock().unwrap();
                    let current = identity.scope == state.scope;
                    Ok(ViewContext {
                        today: date(),
                        signed_in: identity.signed_in,
                        account: if current && state.mode == DataSyncMode::Enabled {
                            state.scope.as_ref().map(|v| v.account.clone())
                        } else {
                            None
                        },
                        sync_mode: if current {
                            state.mode
                        } else {
                            DataSyncMode::Evaluating
                        },
                    })
                }),
                configure(owner, "statistics"),
                events.clone(),
            );
            let local = Arc::new(Repositories {
                dictionary,
                styles,
                statistics,
            });
            let journals = Arc::new(SyncJournalRepository::new(store.clone()));
            let weak = owner.clone();
            let context: super::sync::Context =
                Arc::new(move || weak.upgrade().and_then(|host| host.sync_scope()));
            // UI admission and sync capture/application use the identical gate.
            let gate = Arc::new(tokio::sync::Mutex::new(()));
            let workflow = SyncDocuments::new(
                journals.clone(),
                gate.clone(),
                local.clone(),
                transport.clone(),
                context.clone(),
                now.clone(),
                today.clone(),
            );
            let weak = owner.clone();
            let stages = Arc::new(move |stage, scope, cancellation| {
                let workflow = workflow.clone();
                let weak = weak.clone();
                Box::pin(async move {
                    workflow.run(stage, scope, cancellation).await?;
                    if stage == Stage::Statistics {
                        weak.upgrade()
                            .ok_or("documents_stopped")?
                            .update_pending()
                            .await?;
                    }
                    Ok(())
                }) as super::sync::StageFuture
            });
            let weak = owner.clone();
            let driver = SyncDriver::new(
                context,
                stages,
                Arc::new(move |_| {
                    weak.upgrade()
                        .is_some_and(|h| h.pending.load(Ordering::SeqCst))
                }),
                diagnostics,
                Arc::new(|| 0.5),
            );
            let view = StylesFeature::new(
                local.styles.clone(),
                style_facts,
                Arc::new(move |generation| format!("{instance}:{generation}")),
                events,
            );
            Self {
                local,
                styles: view,
                enrollment: SyncEnrollmentRepository::new(store),
                journals,
                driver,
                gate,
                state: Mutex::new(State {
                    scope: None,
                    mode: DataSyncMode::LocalOnly,
                    resolved_identity: None,
                }),
                identity,
                today,
                port,
                pending: AtomicBool::new(false),
                stopped: AtomicBool::new(false),
            }
        })
    }
    fn sync_scope(&self) -> Option<Scope> {
        if self.stopped.load(Ordering::SeqCst) {
            return None;
        }
        let state = self.state.lock().unwrap();
        if state.mode == DataSyncMode::Enabled && (self.identity)().scope == state.scope {
            state.scope.clone()
        } else {
            None
        }
    }
    async fn configured(&self, kind: &'static str, value: Value) -> Result<(), &'static str> {
        // The mutation gate retains the admitted account until its durable
        // notification finishes. Auth changes alone cannot retarget this edit.
        let scope = self.state.lock().unwrap().scope.clone();
        (self.port)(kind, value.clone()).await?;
        if value["localChange"] == true {
            if kind == "styles"
                && value["contentDirty"] != true
                && value["preferencesDirty"] != true
            {
                return Ok(());
            }
            match kind {
                "dictionary" => self
                    .journals
                    .mark_content_changed()
                    .await
                    .map_err(|_| "sync_storage_unavailable")?,
                "styles" => {
                    if value["contentDirty"] == true {
                        self.journals
                            .mark_content_changed()
                            .await
                            .map_err(|_| "sync_storage_unavailable")?;
                    }
                    if let Some(scope) = scope.filter(|_| value["preferencesDirty"] == true) {
                        let context = &value["context"];
                        let local = json!({"defaultStyle":context["state"]["selectedStyle"],
                            "punctuationMode":context["currentOptions"]["punctuationMode"],"emojiMode":context["currentOptions"]["emojiMode"],
                            "profanityFilterEnabled":context["currentOptions"]["profanityFilterEnabled"]});
                        self.journals
                            .append_preference(
                                &scope.account,
                                json!({"kind":"replace","local":local}),
                            )
                            .await
                            .map_err(|_| "sync_storage_unavailable")?;
                    }
                }
                "statistics" => (),
                _ => return Err("documents_invalid_configuration"),
            }
            self.pending.store(true, Ordering::SeqCst);
            self.driver.request(false);
        }
        Ok(())
    }
    pub fn mode(&self) -> DataSyncMode {
        let state = self.state.lock().unwrap();
        if (self.identity)().scope == state.scope {
            state.mode
        } else {
            DataSyncMode::Evaluating
        }
    }
    pub fn request_sync(&self) {
        self.driver.request(true);
    }
    pub async fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.styles.stop().await;
        self.driver.stop().await;
        // Drain admitted storage/configuration writes before the agent pipe closes.
        let _gate = self.gate.lock().await;
    }
    pub fn attach_style_effects(&self, io: super::styles::Io) -> Result<(), &'static str> {
        self.styles.attach_effects(super::styles::Effects {
            io,
            mutation_gate: self.gate.clone(),
        })
    }
    async fn update_pending(&self) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        let Some(scope) = self.sync_scope() else {
            return Ok(());
        };
        let content = self
            .journals
            .capture(
                govorun_core::sync_journal::JournalKind::Content,
                &scope.account,
            )
            .await
            .map_err(|_| "sync_storage_unavailable")?;
        let preferences = self
            .journals
            .capture(
                govorun_core::sync_journal::JournalKind::Preferences,
                &scope.account,
            )
            .await
            .map_err(|_| "sync_storage_unavailable")?;
        let stats = self
            .local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await?;
        let pending = content.has_pending()
            || preferences.has_pending()
            || stats["configuration"]["result"]["projection"]["accounts"][&scope.account]
                ["hasPending"]
                == true;
        self.pending.store(pending, Ordering::SeqCst);
        Ok(())
    }
}
fn configure(owner: &Weak<DocumentsHost>, kind: &'static str) -> Configure {
    let owner = owner.clone();
    Arc::new(move |value| {
        let owner = owner.clone();
        Box::pin(async move {
            owner
                .upgrade()
                .ok_or("documents_stopped")?
                .configured(kind, value)
                .await
        })
    })
}
