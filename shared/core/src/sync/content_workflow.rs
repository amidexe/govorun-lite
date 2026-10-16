//! Content synchronization decisions. Native ports persist each decision before IO.
use crate::content_sync::{merge, validate, Document};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    schema_version: u32,
    base: Document,
    etag: String,
    checked_at: Option<u64>,
    in_flight: Option<Write>,
}

impl Default for State {
    fn default() -> Self {
        Self { schema_version: 1, base: Document::default(), etag: "\"0\"".into(),
            checked_at: None, in_flight: None }
    }
}

/// Both snapshots are durable: an uncertain write must not erase a later local revert.
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Write { local: Document, candidate: Document }

#[derive(Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Stage { Read, Write }

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Flow { stage: Stage, writes: u8, conditional: bool }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Start { #[serde(default)] force: bool },
    Read { not_modified: bool, configured: bool, etag: String, document: Document },
    Written { etag: String, document: Document },
    Failed { status: Option<u16> },
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Command {
    state: Option<State>, flow: Option<Flow>, local: Document, event: Event, now_millis: u64,
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let Command { state, flow, local, event, now_millis } = command;
    let mut state = state.unwrap_or_default();
    validate(&local)?;
    validate(&state.base)?;
    etag(&state.etag)?;
    if state.schema_version != 1 { return Err(Error::InvalidRequest); }
    if let Some(write) = &state.in_flight {
        validate(&write.local)?;
        validate(&write.candidate)?;
    }
    if let Some(flow) = &flow {
        if flow.writes > 3 || (flow.stage == Stage::Write && (flow.writes == 0 || state.in_flight.is_none())) {
            return Err(Error::InvalidRequest);
        }
    }
    match event {
        Event::Start { force } => {
            if flow.is_some() { return Err(Error::InvalidRequest); }
            let recent = state.checked_at.is_some_and(|time| now_millis >= time && now_millis - time < 300_000);
            if !force && !pending(&state, &local) && recent {
                return Ok(result(&state, &local, "complete", None));
            }
            let conditional = state.in_flight.is_none();
            Ok(read(&state, &local, 0, conditional))
        }
        Event::Read { not_modified, configured, etag: received_etag, document } => {
            let flow = flow.filter(|f| f.stage == Stage::Read).ok_or(Error::InvalidRequest)?;
            if not_modified && !flow.conditional { return Err(Error::InvalidResponse); }
            let (remote, remote_etag) = if not_modified { (state.base.clone(), state.etag.clone()) }
                else {
                    etag(&received_etag)?;
                    validate(&document)?;
                    if !configured && document != Document::default() { return Err(Error::InvalidResponse); }
                    (document, received_etag)
                };
            let candidate = if let Some(write) = &state.in_flight {
                let rebased = merge(state.base.clone(), write.candidate.clone(), remote.clone());
                merge(write.local.clone(), local.clone(), rebased)
            } else { merge(state.base.clone(), local.clone(), remote.clone()) };
            validate(&candidate)?;
            if candidate == remote {
                return finish(state, local, candidate, remote, remote_etag, now_millis);
            }
            if flow.writes >= 3 { return Err(Error::InvalidRequest); }
            state.base = remote;
            state.etag = remote_etag;
            state.in_flight = Some(Write { local: local.clone(), candidate: candidate.clone() });
            let mut decision = result(&state, &local, "write", Some(Flow {
                stage: Stage::Write, writes: flow.writes + 1, conditional: false,
            }));
            decision["etag"] = json!(state.etag);
            decision["document"] = json!(candidate);
            Ok(decision)
        }
        Event::Written { etag: received_etag, document } => {
            let _ = flow.filter(|f| f.stage == Stage::Write).ok_or(Error::InvalidRequest)?;
            etag(&received_etag)?;
            validate(&document)?;
            let write = state.in_flight.as_ref().ok_or(Error::InvalidRequest)?;
            let applied = merge(write.local.clone(), local.clone(), document.clone());
            validate(&applied)?;
            finish(state, local, applied, document, received_etag, now_millis)
        }
        Event::Failed { status } => {
            let flow = flow.ok_or(Error::InvalidRequest)?;
            if flow.stage == Stage::Write && status == Some(412) && flow.writes < 3 {
                Ok(read(&state, &local, flow.writes, false))
            } else {
                let mut decision = result(&state, &local, "failed", None);
                decision["status"] = json!(status);
                Ok(decision)
            }
        }
    }
}

fn etag(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.bytes().any(|c| c < 32 || c == 127) {
        Err(Error::InvalidResponse)
    } else { Ok(()) }
}

fn pending(state: &State, local: &Document) -> bool {
    state.in_flight.is_some() || local != &state.base
}

fn result(state: &State, local: &Document, action: &str, flow: Option<Flow>) -> Value {
    json!({"state":state, "hasPending":pending(state, local), "action":action, "flow":flow})
}

fn read(state: &State, local: &Document, writes: u8, conditional: bool) -> Value {
    let mut decision = result(state, local, "read", Some(Flow { stage: Stage::Read, writes, conditional }));
    decision["etag"] = if conditional { json!(state.etag) } else { Value::Null };
    decision
}

fn finish(mut state: State, local: Document, applied: Document, remote: Document,
          etag: String, now_millis: u64) -> Result<Value> {
    state.base = remote;
    state.etag = etag;
    state.checked_at = Some(now_millis);
    state.in_flight = None;
    let mut decision = result(&state, &applied, "complete", None);
    if applied != local { decision["apply"] = json!(applied); }
    Ok(decision)
}
