//! Persist common sync decisions before HTTP; apply under the document gate.
//! Private documents, account scopes and receipts never become renderer events.
#[path="documents_workflow/repositories.rs"]
pub mod repositories;
use super::sync::{Context, Failure, Scope, Stage};
use crate::features::cancellation::Cancellation;
use govorun_core::{
    account_protocol,
    sync_journal::{JournalError, JournalKind, SyncJournalRepository},
    sync_wire, AccountSession,
};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::Mutex;

pub type Result = std::result::Result<Value, &'static str>;
pub type Work<'a> = Pin<Box<dyn Future<Output = Result> + Send + 'a>>;
/// Implementations use the same repositories as UI commands. The enclosing gate
/// covers capture, local read/application and commit, but never a network wait.
pub trait Local: Send + Sync {
    fn snapshot<'a>(&'a self, stage: Stage, scope: &'a Scope) -> Work<'a>;
    fn apply<'a>(&'a self, stage: Stage, scope: &'a Scope, value: Value) -> Work<'a>;
    fn statistics<'a>(&'a self, scope: &'a Scope, event: Value) -> Work<'a>;
}
pub trait Transport: Send + Sync {
    fn call(&self, command: Value) -> Work<'_>;
}
pub struct AccountTransport(pub AccountSession);
impl Transport for AccountTransport {
    fn call(&self, command: Value) -> Work<'_> {
        Box::pin(async move {
            account_protocol::dispatch(&self.0, command)
                .await
                .map_err(|e| e.code())
        })
    }
}
pub struct SyncDocuments {
    journals: Arc<SyncJournalRepository>,
    gate: Arc<Mutex<()>>,
    local: Arc<dyn Local>,
    transport: Arc<dyn Transport>,
    context: Context,
    now: Arc<dyn Fn() -> u64 + Send + Sync>,
    today: Arc<dyn Fn() -> String + Send + Sync>,
}
impl SyncDocuments {
    pub fn new(
        journals: Arc<SyncJournalRepository>,
        gate: Arc<Mutex<()>>,
        local: Arc<dyn Local>,
        transport: Arc<dyn Transport>,
        context: Context,
        now: Arc<dyn Fn() -> u64 + Send + Sync>,
        today: Arc<dyn Fn() -> String + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            journals,
            gate,
            local,
            transport,
            context,
            now,
            today,
        })
    }
    fn check(
        &self,
        scope: &Scope,
        cancellation: &Cancellation,
    ) -> std::result::Result<(), &'static str> {
        if cancellation.is_cancelled() || (self.context)().as_ref() != Some(scope) {
            Err("sync_cancelled")
        } else {
            Ok(())
        }
    }
    pub async fn run(
        &self,
        stage: Stage,
        scope: Scope,
        cancellation: Cancellation,
    ) -> std::result::Result<(), Failure> {
        match stage {
            Stage::Content | Stage::Preferences => {
                self.document(stage, &scope, &cancellation).await
            }
            Stage::Statistics => self.statistics(&scope, &cancellation).await,
        }
    }
    async fn decision(
        &self,
        stage: Stage,
        scope: &Scope,
        cancellation: &Cancellation,
        flow: Value,
        event: Value,
    ) -> Result {
        let _gate = self.gate.lock().await;
        self.check(scope, cancellation)?;
        let kind = if stage == Stage::Content {
            JournalKind::Content
        } else {
            JournalKind::Preferences
        };
        let capture = self
            .journals
            .capture(kind, &scope.account)
            .await
            .map_err(journal_error)?;
        let local = self.local.snapshot(stage, scope).await?;
        self.check(scope, cancellation)?;
        let prepared = match stage {
            Stage::Content => {
                self.journals
                    .prepare_content(capture, local, flow, event, (self.now)())
            }
            _ => self
                .journals
                .prepare_preferences(capture, local, flow, event),
        }
        .map_err(journal_error)?;
        let decision = prepared.decision().clone();
        if !decision["apply"].is_null() {
            self.local
                .apply(stage, scope, decision["apply"].clone())
                .await?;
        }
        // Once application is admitted, drain its journal ACK even if a logout
        // arrives while writing. The captured account never changes mid-commit.
        self.journals
            .commit(prepared)
            .await
            .map_err(journal_error)?;
        Ok(decision)
    }
    async fn document(
        &self,
        stage: Stage,
        scope: &Scope,
        cancellation: &Cancellation,
    ) -> std::result::Result<(), Failure> {
        let mut failure = Failure::from("sync_transport_failed");
        let mut decision = self
            .decision(
                stage,
                scope,
                cancellation,
                Value::Null,
                json!({"kind":"start"}),
            )
            .await?;
        loop {
            match decision["action"].as_str() {
                Some("complete") => return Ok(()),
                Some("failed") => return Err(failure),
                Some("read" | "write" | "patch") => (),
                _ => return Err("sync_invalid_response".into()),
            }
            self.check(scope, cancellation)?;
            let response = self.transport.call(request(stage, &decision, scope)?).await;
            // A response from a former account must not apply to local documents
            // or acknowledge its durable in-flight receipt after logout.
            self.check(scope, cancellation)?;
            let event = match response {
                Ok(response) => match response_event(stage, &decision, &response) {
                    Ok(event) => event,
                    Err(_) => json!({"kind":"failed","status":response["status"].as_u64()}),
                },
                Err(_) => json!({"kind":"failed","status":null}),
            };
            if event["kind"] == "failed" {
                failure = Failure::http("sync_transport_failed", &event);
            }
            decision = self
                .decision(stage, scope, cancellation, decision["flow"].clone(), event)
                .await?;
        }
    }
    async fn ledger(&self, scope: &Scope, cancellation: &Cancellation, event: Value) -> Result {
        let _gate = self.gate.lock().await;
        self.check(scope, cancellation)?;
        self.local.statistics(scope, event).await
    }
    async fn statistics(
        &self,
        scope: &Scope,
        cancellation: &Cancellation,
    ) -> std::result::Result<(), Failure> {
        let prepare = || json!({"kind":"prepare","account":scope.account,"today":(self.today)()});
        let mut decision = self.ledger(scope, cancellation, prepare()).await?;
        if decision["action"] == "delete" {
            self.check(scope, cancellation)?;
            let response = self
                .transport
                .call(json!({"operation":"syncStatisticsDelete",
                "sessionGeneration":scope.generation,"confirmedReset":true}))
                .await?;
            if !successful(&response) {
                return Err(Failure::http("stats_sync_failed", &response));
            }
            self.ledger(
                scope,
                cancellation,
                json!({"kind":"deleteAccepted","account":scope.account,"epoch":decision["epoch"]}),
            )
            .await?;
            decision = self.ledger(scope, cancellation, prepare()).await?;
        }
        if decision["action"] != "sync" {
            return Err("statistics_invalid_response".into());
        }
        self.check(scope, cancellation)?;
        let response = self
            .transport
            .call(json!({"operation":"syncStatisticsWrite",
            "sessionGeneration":scope.generation,"request":decision["request"]}))
            .await?;
        if !successful(&response) {
            return Err(Failure::http("stats_sync_failed", &response));
        }
        self.ledger(
            scope,
            cancellation,
            json!({"kind":"accept","account":scope.account,
            "receipt":decision["receipt"],"response":response["body"]}),
        )
        .await?;
        Ok(())
    }
}
fn successful(response: &Value) -> bool {
    response["status"]
        .as_u64()
        .is_some_and(|s| (200..300).contains(&s))
}
fn request(stage: Stage, decision: &Value, scope: &Scope) -> Result {
    let operation = match (stage, decision["action"].as_str()) {
        (Stage::Content, Some("read")) => "syncContentRead",
        (Stage::Content, Some("write")) => "syncContentWrite",
        (Stage::Preferences, Some("read")) => "syncPreferencesRead",
        (Stage::Preferences, Some("patch")) => "syncPreferencesPatch",
        _ => return Err("sync_invalid_response"),
    };
    let mut request = json!({"operation":operation,"sessionGeneration":scope.generation});
    if stage == Stage::Content {
        request["etag"] = decision["etag"].clone();
        if decision["action"] == "write" {
            request["document"] = decision["document"].clone();
        }
    } else if decision["action"] == "patch" {
        request["patch"] = decision["patch"].clone();
    }
    Ok(request)
}
fn response_event(stage: Stage, decision: &Value, response: &Value) -> Result {
    if stage == Stage::Content {
        let writing = decision["action"] == "write";
        let admitted = sync_wire::content_response(response, decision["etag"].as_str(), writing)
            .map_err(|_| "sync_invalid_response")?;
        if writing {
            Ok(json!({"kind":"written","etag":admitted.etag,"document":admitted.document}))
        } else {
            Ok(json!({"kind":"read","notModified":admitted.not_modified,
            "configured":admitted.configured,"etag":admitted.etag,"document":admitted.document}))
        }
    } else if response["status"] == 200 {
        Ok(json!({"kind":"document","document":response["body"]}))
    } else {
        Err("sync_transport_failed")
    }
}
fn journal_error(error: JournalError) -> &'static str {
    match error {
        JournalError::InvalidRequest => "sync_invalid_request",
        JournalError::InvalidStorage => "sync_storage_invalid",
        JournalError::Unavailable => "sync_storage_unavailable",
        JournalError::Conflict => "sync_conflict",
    }
}
