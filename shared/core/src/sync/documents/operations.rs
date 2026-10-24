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
        let dictionary = self.local.dictionary.execute(json!({"kind":"snapshot"}), None, false).await;
        let styles = self.local.styles.execute(json!({"kind":"snapshot"}), None, false).await;
        let statistics = self.local.statistics.execute(json!({"kind":"inspect"}), false, false).await;
        // A document that cannot be opened is named once per launch; its own page shows it.
        for (result, event) in [(&dictionary, "dictionary_load_failed"), (&styles, "settings_load_failed"),
            (&statistics, "statistics_journal_unavailable")] {
            if let Err(code) = result.as_ref().map(|_| ()) {
                stopped(code)?;
                self.report_once(event, code);
            }
        }
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
        // The decision asks only whether local data exists. Upload limits, a
        // read-only statistics journal or an unreadable sync journal belong to the
        // stage that uses them and are reported there; they never hold sign-in.
        // A document that cannot be opened counts as local data: the person is asked.
        let mut unread = false;
        let content = match self.local.content().await {
            Ok(content) => content,
            Err(code) => {
                stopped(code)?;
                unread = true;
                json!({"schemaVersion":1,"dictionary":{"enabled":false,"text":""},
                    "selectedCustomStyleId":"","customStyles":[]})
            }
        };
        let statistics = match self.local.statistics.execute(json!({"kind":"inspect"}), false, false).await {
            Ok(stats) => {
                if stats["configuration"]["writable"] != true {
                    self.report_once("statistics_journal_unavailable", "statistics_storage_invalid");
                }
                let projection = &stats["configuration"]["result"]["projection"]["local"];
                json!({"total":projection["total"],"days":projection["days"]})
            }
            Err(code) => {
                stopped(code)?;
                unread = true;
                json!({"total":{"words":0,"speechSeconds":0,"recognitions":0},"days":[]})
            }
        };
        let mut history = false;
        for kind in [JournalKind::Content, JournalKind::Preferences] {
            match self.journals.capture(kind, &scope.account).await {
                Ok(journal) => history |= journal.has_history(),
                Err(_) => self.report_once("sync_state_load_failed", "sync_storage_unavailable"),
            }
        }
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
                    history,
                    remote_data,
                    unread,
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
        // Republish the account view; its own failure never cancels the sync start.
        let _ = self
            .local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await;
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
            // Unreadable styles leave no preference intent; their stage reports itself.
            if let Ok(local) = self.local.snapshot(Stage::Preferences, &scope).await {
                self.journals
                    .append_preference(&scope.account, json!({"kind":"replace","local":local}))
                    .await
                    .map_err(|_| "sync_storage_unavailable")?;
            }
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
        let _ = self
            .local
            .statistics
            .execute(json!({"kind":"inspect"}), false, false)
            .await;
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
/// Shutdown and an account change end the attempt; any other code is local data.
fn stopped(code: &'static str) -> Result<(), &'static str> {
    if matches!(code, "documents_stopped" | "account_session_changed") {
        Err(code)
    } else {
        Ok(())
    }
}
