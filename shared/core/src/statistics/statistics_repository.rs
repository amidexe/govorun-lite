//! Single writer for the current statistics envelope and numeric agent facts.
#[path="statistics_repository/continuity.rs"]
mod continuity;
use crate::{DocumentStore, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatisticsDocumentError { InvalidAction, InvalidStorage, StorageUnavailable, Conflict }
impl From<StoreError> for StatisticsDocumentError {
    fn from(error: StoreError) -> Self {
        match error { StoreError::Invalid => Self::InvalidStorage,
            StoreError::Unavailable => Self::StorageUnavailable, StoreError::Conflict => Self::Conflict }
    }
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Physical { stored: Option<String>, continuity: Value }
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope { schema_version: u32, state: Value, projection: Value }
pub struct StatisticsContext { pub today: String, pub account: Option<String> }
pub struct StatisticsDocumentResult { pub result: Value, pub revision: String, pub changed: bool, pub writable: bool }
pub struct StatisticsRepository { store: Arc<dyn DocumentStore>, serial: Mutex<()> }
impl StatisticsRepository {
    pub fn new(store: Arc<dyn DocumentStore>) -> Self { Self { store, serial: Mutex::new(()) } }

    pub async fn execute(&self, event: Value, context: StatisticsContext, confirmed_reset: bool)
        -> Result<StatisticsDocumentResult, StatisticsDocumentError> {
        use crate::statistics_ledger::{Action, Event};
        let action: Action = serde_json::from_value(event.clone()).map_err(|_| StatisticsDocumentError::InvalidAction)?;
        // The document port receives numeric facts only. Text-based legacy
        // events remain in the pure API used by unchanged Android clients.
        match action {
            Action::Inspect if event.as_object().is_none_or(|value| value.len() != 1) => return Err(StatisticsDocumentError::InvalidAction),
            Action::Reset { .. } if !confirmed_reset => return Err(StatisticsDocumentError::InvalidAction),
            Action::CountWords { .. } | Action::Record { event: Event::Session { .. } | Event::Recognition { .. }, .. } =>
                return Err(StatisticsDocumentError::InvalidAction),
            _ => (),
        }
        crate::statistics_sync::date(&context.today).map_err(|_| StatisticsDocumentError::InvalidAction)?;
        let _serial = self.serial.lock().await;
        let key = "statistics.document".to_owned();
        let document = self.store.read_document(key.clone()).await?.ok_or(StatisticsDocumentError::InvalidStorage)?;
        if document.revision.is_empty() || document.revision.len() > 128 { return Err(StatisticsDocumentError::InvalidStorage); }
        let mut physical: Physical = serde_json::from_value(document.value).map_err(|_| StatisticsDocumentError::InvalidStorage)?;
        if physical.continuity.get("source").is_some()
            && !matches!(physical.continuity["source"].as_str(), Some("macos" | "windows")) {
            return Err(StatisticsDocumentError::InvalidStorage);
        }
        if physical.stored.as_ref().is_some_and(|raw| raw.len() > 8 * 1_048_576) { return Err(StatisticsDocumentError::InvalidStorage); }
        let decoded = physical.stored.as_deref().map(serde_json::from_str::<Value>).transpose();
        let previous = decoded.as_ref().ok().and_then(|value| value.clone());
        if decoded.is_err() && event["kind"] != "reset" {
            // An unreadable current journal cannot prove whether an explicit
            // reset occurred after the preceding store. Show the readable
            // preceding counters with a warning, but never rewrite or upload
            // them as an authoritative current ledger. Only a confirmed reset
            // can replace the unreadable journal.
            if event["kind"] != "inspect" { return Err(StatisticsDocumentError::InvalidStorage); }
            let facts = previous_facts(&physical.continuity, None, &context.today);
            return recovery_view(facts, document.revision);
        }
        let mut state = match previous.clone() {
            Some(value) => {
                let envelope: Envelope = serde_json::from_value(value).map_err(|_| StatisticsDocumentError::InvalidStorage)?;
                if envelope.schema_version != 1 { return Err(StatisticsDocumentError::InvalidStorage); }
                if envelope.state.get("schemaVersion").is_some_and(|version| version != 1) {
                    return Err(StatisticsDocumentError::InvalidStorage);
                }
                Some(crate::statistics_ledger::persistence::restore(envelope.state, &envelope.projection))
            }
            None if decoded.is_err() => Some(json!({"unreadable":true})),
            None => None,
        };
        let account = event["account"].as_str().map(str::to_owned).or(context.account);
        let restored = state.as_ref().is_some_and(|state| state["continuityImported"] == true);
        let account_restored = account.as_ref().is_none_or(|id| state.as_ref().is_some_and(|state| state["accounts"][id]["continuityImported"] == true));
        let mut continuity = if restored && account_restored || decoded.is_err() { None }
            else { Some(previous_facts(&physical.continuity, account.as_deref(), &context.today)) };
        if continuity.is_none() && decoded.is_ok() && account.is_some() {
            if let Some(current) = state.as_ref() {
                let facts = previous_facts(&physical.continuity, account.as_deref(), &context.today);
                if crate::statistics_continuity::repair_needed(current, &facts) {continuity = Some(facts);}
            }
        }
        if state.is_none() && continuity.as_ref().is_some_and(|value| value["unreadable"] == true) {
            // A transient preceding-store read failure must not create an empty
            // authoritative ledger that permanently hides existing statistics.
            if unavailable_fact(&physical.continuity) { return Err(StatisticsDocumentError::StorageUnavailable); }
            if event["kind"] == "inspect" {
                return recovery_view(continuity.take().unwrap(), document.revision);
            }
            if event["kind"] != "reset" || !confirmed_reset { return Err(StatisticsDocumentError::InvalidStorage); }
            // The existing confirmed reset may establish a new empty journal;
            // it never rewrites or deletes the unreadable preceding bytes.
            state = Some(json!({"unreadable":true}));
            continuity = None;
        }
        let command = serde_json::from_value(json!({"state":state,"continuity":continuity,
            "recoverStorage":true,"event":event})).map_err(|_| StatisticsDocumentError::InvalidAction)?;
        let result = crate::statistics_ledger::evaluate(command).map_err(|_| StatisticsDocumentError::InvalidAction)?;
        let saved = json!({"schemaVersion":1,"state":result["state"],"projection":result["projection"]});
        if previous.as_ref() == Some(&saved) {
            return Ok(StatisticsDocumentResult { result, revision: document.revision, changed: false, writable:true });
        }
        physical.stored = Some(saved.to_string());
        let value = json!({"stored":physical.stored,"continuity":physical.continuity});
        let written = self.store.write_document(key, Some(document.revision), value.clone()).await?;
        if written.value != value || written.revision.is_empty() || written.revision.len() > 128 { return Err(StatisticsDocumentError::InvalidStorage); }
        Ok(StatisticsDocumentResult { result, revision: written.revision, changed: true, writable:true })
    }
}

fn recovery_view(facts: Value, revision: String) -> Result<StatisticsDocumentResult, StatisticsDocumentError> {
    let command = serde_json::from_value(json!({"state":null,"continuity":facts,
        "recoverStorage":true,"event":{"kind":"inspect"}})).map_err(|_| StatisticsDocumentError::InvalidStorage)?;
    let mut result = crate::statistics_ledger::evaluate(command).map_err(|_| StatisticsDocumentError::InvalidStorage)?;
    result["projection"]["storageRecovered"] = json!(true);
    Ok(StatisticsDocumentResult { result, revision, changed:false, writable:false })
}

fn unavailable_fact(value: &Value) -> bool {
    match value {
        Value::Object(fields) => fields.get("unavailable") == Some(&json!(true)) || fields.values().any(unavailable_fact),
        Value::Array(values) => values.iter().any(unavailable_fact),
        _ => false,
    }
}

fn previous_facts(facts: &Value, account: Option<&str>, today: &str) -> Value {
    if facts.get("source").is_some() { return continuity::previous(facts, account, today); }
    let Some(local) = facts["local"].as_object() else { return json!({"unreadable":true}) };
    let account = account.map(|id| facts["accounts"][id].as_object().map(|value| Value::Object(value.clone()))
        .unwrap_or_else(|| json!({"id":id,"generation":0,"cloud":null,"acknowledgedTotal":{
            "words":0,"speechSeconds":0,"recognitions":0},"acknowledgedDays":[],"resetPending":false})));
    json!({"today":today,"total":local.get("total"),"days":local.get("days"),"account":account})
}

/// Text is counted inside the core and never enters the document/store port.
pub fn recording_session_facts(text: &str, speech_milliseconds: i64) -> crate::Result<Value> {
    Ok(json!({"kind":"sessionFacts","words":crate::statistics_ledger::count_words(text),
        "speechSeconds":crate::statistics_ledger::speech_seconds(speech_milliseconds)?}))
}
