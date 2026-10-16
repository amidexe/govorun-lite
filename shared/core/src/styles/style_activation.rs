//! Style selection is a local opt-in, never financial or permission to use the network.
//! Ports provide published account facts and I/O lifetime observations; this
//! owner chooses the gate and whether a confirmed selection may be applied.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
pub(crate) const CHECKING_AVAILABILITY: &str = "Проверяем доступ к стилизации";

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Facts {
    pub generation: u64,
    pub signed_in: bool,
    pub profile_fresh: bool,
    pub storage_available: bool,
    // Dictation permission is reported separately; it never changes online consent.
    #[serde(rename = "permissionsReady")]
    pub _permissions_ready: bool,
    pub terms_match: bool,
    pub entitled: bool,
    pub starter_locally_exhausted: bool,
    pub authorized: bool,
    pub consent_accepted: bool,
    pub processing_ready: bool,
    pub availability: String,
    pub self_hosted: bool,
    pub self_hosted_ready: bool,
    pub client_terms_version: u32,
    pub local_terms_version: u32,
    pub enabled: bool,
    pub paused: bool,
    pub processing_readiness: String,
    pub preset_count: u32,
}

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Presentation {
    pub generation: u64,
    pub gate: &'static str,
    pub availability: Option<&'static str>,
    pub terms_accepted: bool,
    pub restore_consent: bool,
    #[serde(flatten)]
    pub display: crate::style_display::Display,
}

pub(crate) fn project(f: &Facts) -> Result<Presentation> {
    if f.client_terms_version == 0 { return Err(Error::InvalidRequest); }
    let verified_style = f.authorized && f.entitled && f.consent_accepted;
    let confirmed = f.signed_in && (f.profile_fresh || verified_style) && f.storage_available && f.terms_match;
    let accepted = confirmed && (f.consent_accepted || f.authorized);
    let cached_limit = f.signed_in && f.storage_available && f.terms_match && !f.profile_fresh &&
        matches!(f.availability.as_str(), "dailyLimit" | "periodLimit");
    let reason = if (!confirmed || !f.processing_ready) && !cached_limit { "unavailable" }
        else { match f.availability.as_str() {
            "ready" => "ready", "periodLimit" => "periodLimit", "dailyLimit" => "dailyLimit",
            "unavailable" => "unavailable", _ => "checking",
        }};
    let availability = match reason {
        "ready" => None,
        "periodLimit" => Some("Стилизация приостановлена: общий лимит закончился"),
        "dailyLimit" => Some("Стилизация приостановлена: лимит на сегодня закончился"),
        "unavailable" => Some("Стилизация временно недоступна"),
        _ => Some(CHECKING_AVAILABILITY),
    };
    let gate = if f.self_hosted {
        if f.self_hosted_ready { "selfHosted" } else { "selfHostedSetup" }
    } else if !f.storage_available { "storage" }
        else if !f.signed_in { "account" }
        else if !confirmed { "unavailable" }
        else if !f.entitled || f.starter_locally_exhausted { "access" }
        else if !accepted { "terms" }
        else if !f.authorized || availability.is_some() { "unavailable" }
        else { "ready" };
    Ok(Presentation { generation: f.generation, gate, availability,
        terms_accepted: accepted, restore_consent: accepted && f.profile_fresh && f.local_terms_version != f.client_terms_version,
        display: crate::style_display::project(gate, reason, f.enabled, f.paused,
            &f.processing_readiness, f.preset_count, f.self_hosted)? })
}

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Intent {
    pub generation: u64,
    pub operation_id: String,
    pub terms_version: u32,
    pub selection: String,
}

#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Lifetime {
    pub active_operation_id: Option<String>,
    pub cancelled: bool,
    pub disposed: bool,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub(crate) enum Event {
    Begin { operation_id: String, selection: String, selection_exists: bool,
        terms_version: u32, confirmed: bool, accept_terms: bool },
    Complete { intent: Intent, lifetime: Lifetime, selection_exists: bool },
    Failed { generation: u64, operation_id: String, lifetime: Lifetime },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Effect {
    pub kind: &'static str,
    pub selection: Option<String>,
    pub accepted_version: Option<u32>,
    pub message: Option<&'static str>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub(crate) struct Frame { pub intent: Option<Intent>, pub effect: Effect }

pub(crate) fn transition(facts: Facts, event: Event) -> Result<Frame> {
    let presentation = project(&facts)?;
    match event {
        Event::Begin { operation_id, selection, selection_exists, terms_version, confirmed, accept_terms } => {
            // preserves running improvement, not fresh-only consent/configuration
            // mutations. A background observation cannot write a consent mirror.
            if !facts.profile_fresh && !facts.self_hosted { return Err(Error::InvalidRequest); }
            if !confirmed || terms_version != facts.client_terms_version || !selection_exists ||
                operation_id.is_empty() || operation_id.len() > 128 || selection.is_empty() || selection.len() > 256 ||
                (selection == "selfHosted" && !facts.self_hosted) {
                return Err(Error::InvalidRequest);
            }
            if presentation.gate == "ready" {
                return Ok(frame("applySelection", Some(selection), Some(terms_version), None));
            }
            if presentation.gate == "selfHosted" && selection == "selfHosted" {
                return Ok(frame("applySelfHosted", Some(selection), None, None));
            }
            if presentation.gate != "terms" || !accept_terms { return Err(Error::InvalidRequest); }
            // Reuse the existing consent planner. Local settings cannot replace
            // the current account's consent or grant a new allowance.
            let instruction = crate::access_interaction::plan(crate::access_interaction::AccessInteractionFacts {
                signed_in: facts.signed_in, storage_available: facts.storage_available,
                enable_allowed: true, terms_action_available: true,
                consent_accepted: facts.consent_accepted, entitlement_active: facts.entitled,
                authorized: facts.authorized, free_access_available: false,
            }, crate::access_interaction::Action::Terms, facts.local_terms_version == facts.client_terms_version, true, false)?;
            if instruction.kind != "acceptTerms" { return Err(Error::InvalidRequest); }
            let intent = Intent { generation: facts.generation, operation_id, terms_version, selection };
            Ok(Frame { intent: Some(intent), effect: effect("acceptTerms", None, None, None) })
        }
        Event::Complete { intent, lifetime, selection_exists } => {
            if !current(&facts, intent.generation, &intent.operation_id, &lifetime) ||
                intent.terms_version != facts.client_terms_version || !facts.terms_match || facts.self_hosted ||
                intent.selection == "selfHosted" {
                return Ok(frame("none", None, None, None));
            }
            if presentation.gate == "ready" && selection_exists {
                Ok(frame("applySelection", Some(intent.selection), Some(intent.terms_version), None))
            } else {
                Ok(frame("recordConsent", None, Some(intent.terms_version),
                    Some("Условия приняты. Доступ к обработке пока не подтверждён.")))
            }
        }
        Event::Failed { generation, operation_id, lifetime } => {
            Ok(if !facts.self_hosted && current(&facts, generation, &operation_id, &lifetime) {
                frame("failure", None, None, Some("Не удалось выполнить действие. Попробуйте ещё раз."))
            } else { frame("none", None, None, None) })
        }
    }
}

fn current(facts: &Facts, generation: u64, operation_id: &str, lifetime: &Lifetime) -> bool {
    facts.signed_in && !lifetime.cancelled && !lifetime.disposed && facts.generation == generation &&
        lifetime.active_operation_id.as_deref() == Some(operation_id)
}

fn effect(kind: &'static str, selection: Option<String>, accepted_version: Option<u32>,
    message: Option<&'static str>) -> Effect {
    Effect { kind, selection, accepted_version, message }
}

fn frame(kind: &'static str, selection: Option<String>, accepted_version: Option<u32>,
    message: Option<&'static str>) -> Frame {
    Frame { intent: None, effect: effect(kind, selection, accepted_version, message) }
}
