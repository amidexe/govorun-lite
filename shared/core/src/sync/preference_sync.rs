use crate::{
    preference_model::{Changes, InputChanges, Values, WireValues},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    schema_version: u32,
    baseline: Option<Values>,
    pending: Changes,
}
impl Default for State {
    fn default() -> Self {
        Self {
            schema_version: 1,
            baseline: None,
            pending: Changes::default(),
        }
    }
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Phase {
    Read,
    Patch,
}
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Flow {
    phase: Phase,
    conflicts: u8,
    sent: Changes,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Document {
    schema_version: u32,
    configured: bool,
    revision: i64,
    #[serde(rename = "updatedAt")]
    _updated_at: Option<String>,
    values: WireValues,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Intent {
    Change {
        changes: InputChanges,
        local: Values,
    },
    Replace {
        local: Values,
    },
    Compare {
        before: Values,
        after: Values,
    },
}

fn intent_changes(intent: Intent) -> Result<Changes> {
    match intent {
        Intent::Change { changes, local } => {
            local.validate(true)?;
            let change = changes.normalize()?;
            Ok(if local.custom() { Changes::default() } else { change })
        }
        Intent::Replace { local } => {
            local.validate(true)?;
            Ok(if local.custom() { Changes::default() } else { Changes::all(&local) })
        }
        Intent::Compare { before, after } => {
            before.validate(true)?;
            after.validate(true)?;
            Ok(Changes::between(&before, &after))
        }
    }
}

pub(crate) fn validate_intent(value: Value) -> Result<()> {
    intent_changes(serde_json::from_value(value).map_err(|_| Error::InvalidRequest)?).map(|_| ())
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Start,
    Document { document: Document },
    Failed { status: Option<u16> },
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Command {
    state: Option<State>,
    flow: Option<Flow>,
    local: Values,
    intents: Vec<Intent>,
    event: Event,
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let Command {
        state,
        flow,
        local,
        intents,
        event,
    } = command;
    local.validate(true)?;
    let mut state = state.unwrap_or_default();
    if state.schema_version != 1 || intents.len() > 4096 {
        return Err(Error::InvalidRequest);
    }
    state.pending.validate()?;
    if let Some(base) = &state.baseline {
        base.validate(true)?;
    }
    let consumed = intents.len();
    for intent in intents {
        state.pending.overlay(intent_changes(intent)?);
    }
    if let Some(base) = &state.baseline {
        state.pending.overlay(Changes::between(base, &local));
    }
    state.baseline = Some(local.clone());
    let output = match event {
        Event::Start => {
            json!({"action":"read","flow":Flow {phase:Phase::Read,conflicts:0,sent:Changes::default()}})
        }
        Event::Failed { status } => {
            let mut flow = flow.ok_or(Error::InvalidRequest)?;
            if flow.phase == Phase::Patch && status == Some(409) && flow.conflicts == 0 {
                flow.conflicts = 1;
                flow.phase = Phase::Read;
                json!({"action":"read","flow":flow})
            } else {
                json!({"action":"failed","flow":null})
            }
        }
        Event::Document { document } => {
            let mut flow = flow.ok_or(Error::InvalidRequest)?;
            if flow.conflicts > 1
                || document.schema_version != 1
                || document.revision < 0
                || (document.configured && document.revision == 0)
            {
                return Err(Error::InvalidRequest);
            }
            let remote = document.values.canonical()?;
            if flow.phase == Phase::Patch {
                if !document.configured {
                    return Err(Error::InvalidRequest);
                }
                state.pending.acknowledge(&flow.sent);
                complete(&mut state, &local, &remote)
            } else if document.configured && state.pending.empty() {
                complete(&mut state, &local, &remote)
            } else {
                let sent = if document.configured {
                    state.pending.clone()
                } else {
                    let mut all = Changes::all(&local.portable());
                    all.overlay(state.pending.clone());
                    all
                };
                state.pending.overlay(sent.clone());
                let patch = json!({"schemaVersion":1,"baseRevision":document.revision,"changes":sent.wire()?});
                flow.phase = Phase::Patch;
                flow.sent = sent;
                json!({"action":"patch","flow":flow,"patch":patch})
            }
        }
    };
    let mut output = output;
    output["hasPending"] = json!(!state.pending.empty());
    output["state"] = serde_json::to_value(state).map_err(|_| Error::InvalidResponse)?;
    output["consumed"] = json!(consumed);
    Ok(output)
}
fn complete(state: &mut State, local: &Values, remote: &Values) -> Value {
    let apply = (!local.custom())
        .then(|| state.pending.apply(remote))
        .filter(|value| value != local);
    state.baseline = Some(apply.clone().unwrap_or_else(|| local.clone()));
    json!({"action":"complete","flow":null,"apply":apply})
}
