//! Local settings intentions. These never grant server access or confirm a purchase.
use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum RecordingAction { Read, Onboarding, Select }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    Recording { action: RecordingAction, mode: Option<String>, enabled: bool },
    Enable { enabled: bool },
    DismissTerms,
    AcceptTerms { version: u32, accepted_at_millis: u64 },
    Reset,
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::Recording { action, mode, enabled } => {
            let mode = mode.as_deref();
            let mut disabled = false;
            let mut effects = Vec::new();
            let selected = match action {
                RecordingAction::Read => if enabled || mode == Some("repeat_tap") { "repeat_tap" } else { "after_pause" },
                RecordingAction::Onboarding => if mode == Some("after_pause") { "after_pause" } else { "repeat_tap" },
                RecordingAction::Select => {
                    if !matches!(mode, Some("after_pause" | "repeat_tap")) { return Err(Error::InvalidRequest); }
                    disabled = enabled && mode == Some("after_pause");
                    if disabled { effects.extend([json!({"kind":"disableStylization"}), json!({"kind":"preferenceChanged"})]); }
                    effects.push(json!({"kind":"persistRecording"}));
                    mode.unwrap()
                }
            };
            Ok(json!({"mode":selected,"stylizationDisabled":disabled,"effects":effects}))
        }
        Command::Enable { enabled } => Ok(json!({"effects":[
            {"kind":"setEnabled","enabled":enabled}, {"kind":"preferenceChanged"}
        ]})),
        Command::DismissTerms => Ok(json!({"effects":[
            {"kind":"dismissTerms"}, {"kind":"consentScopeChanged"}
        ]})),
        Command::AcceptTerms { version, accepted_at_millis } => {
            if version == 0 || accepted_at_millis == 0 { return Err(Error::InvalidRequest); }
            Ok(json!({"effects":[
                {"kind":"acceptTerms","version":version,"acceptedAtMillis":accepted_at_millis},
                {"kind":"consentScopeChanged"}
            ]}))
        }
        Command::Reset => {
            let defaults = json!({"defaultStyle":"formal","punctuationMode":"normal",
                "shortPhraseThreshold":crate::dictation_plan::MINIMUM,
                "stylizationEnabled":false,"recordingMode":"after_pause"});
            Ok(json!({"effects":[
                {"kind":"beforeReset"}, {"kind":"resetLocal","defaults":defaults},
                {"kind":"preferenceChanged"}, {"kind":"afterReset","defaults":defaults},
                {"kind":"refreshAppearance"}
            ]}))
        }
    }
}
