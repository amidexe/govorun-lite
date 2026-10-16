use super::*;
use crate::features::sync_documents::Local;
use govorun_core::{
    sync_enrollment::{Choice, LocalFacts},
    sync_journal::JournalKind,
};
impl DocumentsHost {
    pub async fn dictation_style(
        &self,
        application: Option<&str>,
        identity: crate::style_repository::ApplicationIdentity,
    ) -> Result<govorun_core::style_repository::DictationStyle, &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local.styles.resolve_dictation(application, identity).await
    }
    pub async fn apply_dictionary(&self, text: &str) -> Result<String, &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local.dictionary.apply_text(text).await
    }
    pub(super) fn admit(&self) -> Result<(), &'static str> {
        if self.stopped.load(Ordering::SeqCst) {
            Err("documents_stopped")
        } else {
            Ok(())
        }
    }
    /// Repair unacknowledged local configuration with its former account still
    /// selected, then resolve the new identity from unchanged persisted consent.
    pub async fn identity_changed(&self, remote_data: Option<bool>) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local
            .dictionary
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        self.local
            .styles
            .execute(json!({"kind":"snapshot"}), None, false)
            .await?;
        self.local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await?;
        let identity = (self.identity)();
        // Profile/status refreshes preserve the account lifetime. Restarting
        // sync for each profile publication would loop after statistics ACK
        // refreshes the profile again. Repair cache ACKs above, then retain the
        // already resolved enrollment until the identity actually changes.
        if self.state.lock().unwrap().resolved_identity.as_ref() == Some(&identity) {
            return Ok(());
        }
        let Some(scope) = identity.scope.clone() else {
            *self.state.lock().unwrap() = State {
                scope: None,
                mode: DataSyncMode::LocalOnly,
                resolved_identity: Some(identity),
            };
            self.driver.request(false);
            return Ok(());
        };
        let content = self.local.snapshot(Stage::Content, &scope).await?;
        let stats = self
            .local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await?;
        if stats["configuration"]["writable"] != true {
            return Err("statistics_storage_invalid");
        }
        let projection = &stats["configuration"]["result"]["projection"]["local"];
        let statistics = json!({"total":projection["total"],"days":projection["days"]});
        let content_history = self
            .journals
            .capture(JournalKind::Content, &scope.account)
            .await
            .map_err(|_| "sync_storage_unavailable")?
            .has_history();
        let preference_history = self
            .journals
            .capture(JournalKind::Preferences, &scope.account)
            .await
            .map_err(|_| "sync_storage_unavailable")?
            .has_history();
        if (self.identity)() != identity {
            return Err("account_session_changed");
        }
        let mode = self
            .enrollment
            .resolve(
                &scope.account,
                LocalFacts {
                    content,
                    statistics,
                    history: content_history || preference_history,
                    remote_data,
                },
            )
            .await
            .map_err(|_| "sync_storage_unavailable")?;
        if (self.identity)() != identity {
            return Err("account_session_changed");
        }
        *self.state.lock().unwrap() = State {
            scope: Some(scope),
            mode,
            resolved_identity: Some(identity),
        };
        self.local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await?;
        self.pending.store(true, Ordering::SeqCst);
        self.driver.request(true);
        Ok(())
    }
    pub async fn choose(&self, generation: u64, choice: Choice) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        let scope = self
            .state
            .lock()
            .unwrap()
            .scope
            .clone()
            .ok_or("account_signed_out")?;
        if scope.generation != generation || (self.identity)().scope.as_ref() != Some(&scope) {
            return Err("account_session_changed");
        }
        if matches!(choice, Choice::Merge) {
            let local = self.local.snapshot(Stage::Preferences, &scope).await?;
            self.journals
                .append_preference(&scope.account, json!({"kind":"replace","local":local}))
                .await
                .map_err(|_| "sync_storage_unavailable")?;
            self.journals
                .mark_content_changed()
                .await
                .map_err(|_| "sync_storage_unavailable")?;
        }
        let mode = self
            .enrollment
            .choose(&scope.account, choice)
            .await
            .map_err(|_| "sync_storage_unavailable")?;
        if (self.identity)().scope.as_ref() != Some(&scope) {
            return Err("account_session_changed");
        }
        self.state.lock().unwrap().mode = mode;
        self.local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await?;
        self.pending
            .store(mode == DataSyncMode::Enabled, Ordering::SeqCst);
        self.driver.request(true);
        Ok(())
    }
    pub async fn invoke(&self, method: &str, params: Value) -> Result<Value, &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        if method.starts_with("dictionary.") {
            self.local.dictionary.invoke(method, params).await
        } else if method.starts_with("statistics.") {
            self.local.statistics.invoke(method, params).await
        } else if method == "styles.snapshot" && params.as_object().is_some_and(|v| v.is_empty()) {
            self.styles.snapshot().await
        } else if method == "styles.action" {
            self.styles.action(params).await
        } else {
            Err("unsupported_command")
        }
    }
    pub async fn publish_styles(&self) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.styles.publish().await
    }
    pub async fn disable_official_selection(&self) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local
            .styles
            .execute(json!({"kind":"setEnabled","enabled":false}), None, true)
            .await?;
        Ok(())
    }
    pub async fn activation_options(
        &self,
        generation: u64,
        supplied: Value,
    ) -> Result<Value, &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.styles.activation_options(generation, supplied).await
    }
    pub async fn apply_subscription(
        &self,
        generation: u64,
        effect: &Value,
    ) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.styles.apply_subscription(generation, effect).await
    }
    pub async fn dismiss_starter_guidance(
        &self,
        is_current: impl Fn() -> bool,
    ) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        if !is_current() {
            return Err("account_session_changed");
        }
        self.local
            .styles
            .execute(json!({"kind":"dismissStarterGuidance"}), None, true)
            .await?;
        Ok(())
    }
    /// These extra numbers ride the next ordinary sync; recording them never
    /// starts a separate request or timer. The following dictation session facts
    /// already wake the existing sync owner.
    pub async fn record_style_processing(&self, account: String, failed: bool, slow: bool) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local.statistics.execute(json!({"kind":"record","today":(self.today)(),
            "event":{"kind":"styleProcessing","account":account,"failed":failed,"slow":slow}}), false, false).await?;
        Ok(())
    }
    pub async fn record(&self, event: Value) -> Result<(), &'static str> {
        let _gate = self.gate.lock().await;
        self.admit()?;
        self.local
            .statistics
            .execute(
                json!({"kind":"record","today":(self.today)(),"event":event}),
                false,
                true,
            )
            .await?;
        Ok(())
    }
}
