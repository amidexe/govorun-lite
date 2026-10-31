use crate::{
    dictation_plan::{self, Access, Command, Plan, Settings},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    phase: Phase,
    plan: Option<Plan>,
    /// Говорун.Эхо's итоги: no draft or own-server route, no app rule, their own length and price.
    #[serde(default)]
    digest: bool,
    /// Another product's request (Говорун.Эхо's improvement or итоги): Govorun's own server only, and the person's
    /// dictation choices — the improvement switch, its style, «Стили для приложений» — do not apply; the terms still do.
    #[serde(default)]
    product: bool,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
enum Phase {
    #[default]
    Begin,
    Draft,
    Route,
    Official,
    Configuration,
    Selection,
    Preparation,
    Network,
    Response,
    Done,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Event {
    Begin {
        streaming: bool,
        bypass: bool,
        #[serde(default)]
        digest: bool,
        #[serde(default)]
        product: bool,
    },
    Draft {
        active: bool,
    },
    Route {
        self_hosted: bool,
        enabled: bool,
    },
    Official {
        units: Vec<u16>,
        enabled: bool,
        accepted_terms: u32,
        current_terms: u32,
        access: Access,
    },
    Configuration {
        configured: bool,
    },
    Selection {
        enabled: bool,
    },
    Prepared {
        access: Access,
    },
    Network {
        available: Option<bool>,
    },
    Response {
        input: Vec<u16>,
        output: Vec<u16>,
    },
    Failure {
        reason: String,
    },
}

/// A single official-processing use case. Native executors only satisfy the
/// requested effect and return facts; no text lives in its continuation.
pub(crate) fn transition(state: Option<State>, event: Event) -> Result<Value> {
    let mut state = state.unwrap_or_default();
    if state.phase == Phase::Done {
        return Err(Error::InvalidRequest);
    }
    let mut outcome = "";
    let mut reason = String::new();
    let action = match event {
        Event::Begin { streaming, bypass, digest, product } => {
            require(&state, Phase::Begin)?;
            state.digest = digest;
            state.product = product || digest;
            if bypass || streaming {
                outcome = "local";
                reason = if bypass { "bypassed" } else { "streamingMode" }.into();
                "complete"
            } else if state.product {
                // Another product's requests go to Govorun's own server only; a custom-style draft does not concern them.
                state.phase = Phase::Official;
                "readOfficial"
            } else {
                state.phase = Phase::Draft;
                "readDraft"
            }
        }
        Event::Draft { active } => {
            require(&state, Phase::Draft)?;
            if active {
                outcome = "local";
                reason = "customStyleDraft".into();
                "complete"
            } else {
                state.phase = Phase::Route;
                "readRoute"
            }
        }
        Event::Route {
            self_hosted,
            enabled,
        } => {
            require(&state, Phase::Route)?;
            if self_hosted && enabled {
                state.phase = Phase::Done;
                "selfHosted"
            } else if self_hosted {
                outcome = "local";
                reason = "stylizationDisabled".into();
                "complete"
            } else {
                state.phase = Phase::Official;
                "readOfficial"
            }
        }
        Event::Official {
            units,
            enabled,
            accepted_terms,
            current_terms,
            access,
        } => {
            require(&state, Phase::Official)?;
            let plan = dictation_plan::plan(
                &units,
                Settings {
                    bypass: false,
                    streaming: false,
                    draft: false,
                    self_hosted_selected: false,
                    self_hosted_enabled: false,
                    official_enabled: enabled || state.product,
                    application_enabled: true,
                    accepted_terms,
                    current_terms,
                    digest: state.digest,
                },
            );
            let decision = dictation_plan::admit(plan.clone(), for_request(access, state.product));
            // A temporarily stale ready profile may be repaired by the existing
            // preparation port. Definitive access/consent/length failures stop here.
            let can_prepare = plan.prepare;
            state.plan = Some(plan);
            if decision["action"] == "request"
                || (can_prepare && decision["reason"] == "requestFailed")
            {
                state.phase = Phase::Configuration;
                "readConfiguration"
            } else {
                outcome = if decision["action"] == "local" {
                    "local"
                } else {
                    "fallback"
                };
                reason = decision["reason"]
                    .as_str()
                    .unwrap_or("requestFailed")
                    .into();
                "complete"
            }
        }
        Event::Configuration { configured } => {
            require(&state, Phase::Configuration)?;
            if configured {
                state.phase = Phase::Selection;
                "readSelection"
            } else {
                outcome = "fallback";
                reason = "notConfigured".into();
                "complete"
            }
        }
        Event::Selection { enabled } => {
            require(&state, Phase::Selection)?;
            // «Стили для приложений» chooses how Echo's notes are improved; итоги are no improvement.
            if enabled || state.product {
                state.phase = Phase::Preparation;
                "prepare"
            } else {
                outcome = "local";
                reason = "appRuleDisabled".into();
                "complete"
            }
        }
        Event::Prepared { access } => {
            require(&state, Phase::Preparation)?;
            let plan = state.plan.clone().ok_or(Error::InvalidRequest)?;
            let decision = dictation_plan::admit(plan, for_request(access, state.product));
            if decision["action"] == "request" {
                state.phase = Phase::Network;
                "readNetwork"
            } else {
                outcome = if decision["action"] == "local" {
                    "local"
                } else {
                    "fallback"
                };
                reason = decision["reason"]
                    .as_str()
                    .unwrap_or("requestFailed")
                    .into();
                "complete"
            }
        }
        Event::Network { available } => {
            require(&state, Phase::Network)?;
            if available == Some(false) {
                outcome = "fallback";
                reason = "offline".into();
                "complete"
            } else {
                state.phase = Phase::Response;
                "request"
            }
        }
        Event::Response { input, output } => {
            require(&state, Phase::Response)?;
            let response = dictation_plan::evaluate(Command::Response { input, output })?;
            match response["action"].as_str() {
                Some("changed") => outcome = "styled",
                Some("unchanged") => {
                    outcome = "fallback";
                    reason = "noChange".into();
                }
                _ => {
                    outcome = "fallback";
                    reason = "invalidResponse".into();
                }
            }
            "complete"
        }
        Event::Failure { reason: failure } => {
            outcome = "fallback";
            reason = match failure.as_str() {
                "termsUpdateRequired"
                | "serviceUnavailable"
                | "textTooLong"
                | "notConfigured"
                | "offline"
                | "noChange"
                | "stylePassReserveEmpty"
                | "stylePassIssueFailed"
                | "stylePassRecoveryPending"
                | "accountStateChanged"
                | "transportFailed"
                | "requestFailed"
                | "serverBusy"
                | "serverTimeout"
                | "serverUnavailable"
                | "invalidResponse" => failure,
                _ => "requestFailed".into(),
            };
            "complete"
        }
    };
    if action == "complete" {
        state.phase = Phase::Done;
    }
    let plan = state
        .plan
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| Error::InvalidRequest)?;
    Ok(json!({"state":state,"action":action,"outcome":outcome,"reason":reason,"plan":plan}))
}

fn require(state: &State, phase: Phase) -> Result<()> {
    if state.phase == phase {
        Ok(())
    } else {
        Err(Error::InvalidRequest)
    }
}

/// Another product's request carries its own style, so «a style is chosen for dictation» is not asked of it.
fn for_request(mut access: dictation_plan::Access, product: bool) -> dictation_plan::Access {
    if product {
        access.selected = true;
    }
    access
}
