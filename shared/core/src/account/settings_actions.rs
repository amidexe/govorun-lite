//! Local settings intentions. These never grant server access or confirm a purchase.
use crate::{permission_setup::Pause, Error, Result};
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
    /// Android pause after speech: `choice` is the stored value (`null` when never
    /// chosen), `legacy` the pre-2.0.48 value, `atEnd` whether the text is inserted
    /// after the recording.
    Pause { choice: Option<String>, legacy: Option<String>, at_end: bool },
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
        Command::Pause { choice, legacy, at_end } => {
            // Before 2.0.48 the phone stored «medium» for everyone who never chose:
            // 1.0.8 wrote it over the old default, and a reset writes it too. Only
            // short and long were surely chosen, so they stay; the rest becomes auto.
            let pause = match choice.as_deref().and_then(Pause::parse) {
                Some(pause) => pause,
                None => match legacy.as_deref() {
                    Some("short") => Pause::Short,
                    Some("long") => Pause::Long,
                    _ => Pause::Auto,
                },
            };
            Ok(json!({"pause":pause.name(),"seconds":pause.seconds(at_end)}))
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
