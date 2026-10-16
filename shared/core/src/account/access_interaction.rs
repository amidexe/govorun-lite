use crate::{AccessPresentation, Error, Result};
use serde::{Deserialize, Serialize};

/// Finite access intents; unknown wire actions fail before planning an effect.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "AccessInteractionAction"))]
#[serde(rename_all = "camelCase")]
pub enum Action { Terms, FreeAccess, ResumeFreeAccess, Enable }

impl Action {
    pub fn parse(value: &str) -> Result<Self> {
        serde_json::from_value(serde_json::Value::String(value.into())).map_err(|_| Error::InvalidRequest)
    }
}

/// Published common decisions plus the user's local choices. This plans UI
/// effects only; AccountAccessService still refreshes and validates mutations.
#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessInteractionFacts {
    pub signed_in: bool,
    pub storage_available: bool,
    pub enable_allowed: bool,
    pub terms_action_available: bool,
    pub consent_accepted: bool,
    pub entitlement_active: bool,
    pub authorized: bool,
    pub free_access_available: bool,
}

impl AccessInteractionFacts {
    pub fn published(signed_in: bool, storage_available: bool, access: &AccessPresentation) -> Self {
        Self {
            signed_in,
            storage_available,
            enable_allowed: access.enable_allowed,
            terms_action_available: access.terms_action_available,
            consent_accepted: access.consent_accepted,
            entitlement_active: access.entitlement_active,
            authorized: access.authorized,
            free_access_available: access.free_access_available,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AccessInstruction {
    pub kind: &'static str,
    pub include_terms: bool,
    pub reuse_consent: bool,
    pub already_accepted: bool,
    pub complete_intent: bool,
}

pub fn plan(facts: AccessInteractionFacts, action: Action, local_terms: bool,
    confirmed: bool, pending: bool) -> Result<AccessInstruction> {
    let reusable = facts.signed_in && facts.storage_available && facts.enable_allowed &&
        facts.terms_action_available && facts.consent_accepted && local_terms;
    let instruction = |kind, include_terms| AccessInstruction {
        kind, include_terms, reuse_consent: reusable,
        already_accepted: action == Action::Terms && local_terms,
        complete_intent: action == Action::ResumeFreeAccess &&
            matches!(kind, "completeIntent" | "confirmTerms" | "activateFreeAccess"),
    };
    if action == Action::ResumeFreeAccess && !pending { return Ok(instruction("none", false)); }
    if !facts.signed_in { return Ok(instruction("signIn", false)); }
    if !facts.storage_available || !facts.enable_allowed || !facts.terms_action_available {
        return Ok(instruction("unavailable", false));
    }
    if action == Action::ResumeFreeAccess && (facts.entitlement_active || !facts.free_access_available) {
        return Ok(instruction("completeIntent", false));
    }
    if action == Action::Enable && reusable && facts.authorized {
        return Ok(instruction("applyLocalAccess", false));
    }
    let activate = action != Action::Terms && (action != Action::Enable || !facts.entitlement_active);
    if activate && !facts.free_access_available && !facts.entitlement_active {
        return Ok(instruction("subscription", false));
    }
    let include_terms = action == Action::Terms || !reusable;
    if include_terms && !confirmed { return Ok(instruction("confirmTerms", true)); }
    Ok(instruction(if activate { "activateFreeAccess" } else { "acceptTerms" }, include_terms))
}
