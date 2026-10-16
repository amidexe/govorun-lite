use crate::{style_text, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(crate) const MINIMUM: usize = crate::credit_limits::MINIMUM_CHARACTERS;
pub(crate) const MAXIMUM: usize = crate::credit_limits::MAXIMUM_CHARACTERS;

crate::features::dto::dto!(ProcessingLimits {
    minimum_characters: usize,
    maximum_characters: usize,
    maximum_processing_seconds: u64,
    stop_tail_milliseconds: u64
});

pub(crate) fn processing_limits() -> ProcessingLimits {
    ProcessingLimits {
        stop_tail_milliseconds: crate::recording_controls::STOP_TAIL_NANOS / 1_000_000,
        minimum_characters: MINIMUM,
        maximum_characters: MAXIMUM,
        maximum_processing_seconds: crate::dictation_timing::deadline(MAXIMUM) / 1000,
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Command {
    OnlineProtocol { command: crate::online_protocol::Command },
    Outage {
        state: Option<crate::online_outage::State>,
        event: crate::online_outage::Event,
        now_millis: i64,
    },
    Recording {
        facts: crate::recording_plan::Facts,
    },
    SelfHosted {
        command: crate::self_hosted_flow::Command,
    },
    Transcript {
        state: Option<crate::transcript_flow::State>,
        deferred: bool,
        event: crate::transcript_flow::Event,
    },
    Delivery {
        command: crate::dictation_delivery::Command,
    },
    Workflow {
        state: Option<crate::dictation_flow::State>,
        event: crate::dictation_flow::Event,
    },
    Deadline {
        characters: usize,
    },
    Constants,
    Plan {
        units: Vec<u16>,
        settings: Settings,
    },
    Admit {
        plan: Plan,
        access: Access,
    },
    Response {
        input: Vec<u16>,
        output: Vec<u16>,
    },
}

/// Preferences and OS observations, never credentials or account mutations.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Settings {
    pub bypass: bool,
    pub streaming: bool,
    pub draft: bool,
    pub self_hosted_selected: bool,
    pub self_hosted_enabled: bool,
    pub official_enabled: bool,
    pub application_enabled: bool,
    pub accepted_terms: u32,
    pub current_terms: u32,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Access {
    pub selected: bool,
    pub session_current: bool,
    pub entitlement_active: bool,
    pub processing_ready: bool,
    pub terms_required: bool,
    /// Observations remain in the boundary DTO, but never grant or deny access.
    #[serde(rename = "serviceAvailable")]
    pub _service_available: bool,
    pub storage_available: bool,
    pub configured: bool,
    #[serde(rename = "networkAvailable")]
    pub _network_available: Option<bool>,
}

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Plan {
    route: Route,
    reason: String,
    length: Length,
    characters: usize,
    required_credits: usize,
    deadline_millis: u64,
    pub(crate) prepare: bool,
    terms_required: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Route {
    Local,
    Official,
    SelfHosted,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Length {
    Eligible,
    TooShort,
    TooLong,
    Invalid,
}

/// This local protocol runs before account initialization and never performs I/O.
/// Its continuation contains only decisions/counts; dictated text is not retained.
pub(crate) fn evaluate(command: Command) -> Result<Value> {
    match command {
        Command::OnlineProtocol { command } => crate::online_protocol::evaluate(command),
        Command::Outage { state, event, now_millis } => crate::online_outage::transition(state, event, now_millis),
        Command::Recording { facts } => crate::recording_plan::evaluate(facts),
        Command::SelfHosted { command } => crate::self_hosted_flow::evaluate(command),
        Command::Transcript { state, deferred, event } => crate::transcript_flow::transition(state, deferred, event),
        Command::Delivery { command } => crate::dictation_delivery::evaluate(command),
        Command::Workflow { state, event } => crate::dictation_flow::transition(state, event),
        Command::Deadline { characters } => Ok(json!({"deadlineMillis":crate::dictation_timing::deadline(characters)})),
        Command::Constants => serde_json::to_value(processing_limits()).map_err(|_| Error::InvalidRequest),
        Command::Plan { units, settings } => {
            Ok(serde_json::to_value(plan(&units, settings)).map_err(|_| Error::InvalidRequest)?)
        }
        Command::Admit { plan, access } => Ok(admit(plan, access)),
        Command::Response { input, output } => {
            let input = String::from_utf16(&input).map_err(|_| Error::InvalidRequest)?;
            let output = String::from_utf16(&output).map_err(|_| Error::InvalidRequest)?;
            let output = output.trim();
            let action = if output.is_empty() || output.encode_utf16().count() > 24_000 {
                "invalid"
            } else if input.trim() == output {
                "unchanged"
            } else {
                "changed"
            };
            Ok(json!({"action":action}))
        }
    }
}

pub(crate) fn plan(units: &[u16], settings: Settings) -> Plan {
    let count = style_text::count(units);
    let characters = count.as_ref().copied().unwrap_or(0);
    let length = if count.is_err() {
        Length::Invalid
    } else if characters < MINIMUM {
        Length::TooShort
    } else if characters > MAXIMUM {
        Length::TooLong
    } else {
        Length::Eligible
    };
    let terms_required = settings.accepted_terms != settings.current_terms;
    let (route, reason) = if settings.bypass {
        (Route::Local, "bypassed")
    } else if settings.streaming {
        (Route::Local, "streamingMode")
    } else if settings.draft {
        (Route::Local, "customStyleDraft")
    } else if settings.self_hosted_selected {
        if settings.self_hosted_enabled {
            (Route::SelfHosted, "")
        } else {
            (Route::Local, "stylizationDisabled")
        }
    } else if !settings.official_enabled && !(settings.accepted_terms > 0 && terms_required) {
        (Route::Local, "stylizationDisabled")
    } else if !settings.application_enabled {
        (Route::Local, "appRuleDisabled")
    } else {
        (Route::Official, "")
    };
    let prepare =
        route == Route::Official && settings.official_enabled && length == Length::Eligible;
    let required_credits = if length == Length::Eligible {
        characters.div_ceil(crate::credit_limits::CHARACTERS_PER_CREDIT)
    } else {
        0
    };
    let deadline_millis = crate::dictation_timing::deadline(characters);
    Plan {
        route,
        reason: reason.into(),
        length,
        characters,
        required_credits,
        deadline_millis,
        prepare,
        terms_required,
    }
}

pub(crate) fn admit(plan: Plan, access: Access) -> Value {
    let (action, reason) = if plan.route == Route::Local {
        ("local", plan.reason.as_str())
    } else if plan.route == Route::SelfHosted {
        ("selfHosted", "")
    } else if !access.session_current {
        ("fallback", "accountStateChanged")
    } else if access.terms_required || plan.terms_required {
        ("fallback", "termsUpdateRequired")
    } else if !access.selected {
        ("local", "stylizationDisabled")
    } else if !access.entitlement_active {
        ("local", "accessInactive")
    } else if plan.length == Length::TooShort {
        ("local", "shortPhrase")
    } else if plan.length == Length::TooLong {
        ("fallback", "textTooLong")
    } else if plan.length == Length::Invalid {
        ("fallback", "invalidResponse")
    } else if !access.storage_available {
        ("fallback", "accountStateChanged")
    } else if !access.processing_ready {
        ("fallback", "requestFailed")
    } else if !access.configured {
        ("fallback", "notConfigured")
    } else {
        ("request", "")
    };
    json!({"action":action,"reason":reason,"characters":plan.characters,
        "requiredCredits":plan.required_credits,"deadlineMillis":plan.deadline_millis})
}
