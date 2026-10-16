use crate::{Authority, FreeAccessOffer, Profile, TermsRelation};
use serde::Serialize;

/// UI decisions from the current common session. These flags never grant network
/// authority; requests still pass through the account and access owners.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AccessPresentation {
    pub client_terms_version: u32,
    pub state: &'static str,
    pub enable_allowed: bool,
    pub terms_match: bool,
    pub terms_action_available: bool,
    pub consent_accepted: bool,
    pub needs_terms_acceptance: bool,
    pub entitlement_active: bool,
    /// Last confirmed access for display only; never authorizes a mutation.
    pub display_entitlement_active: bool,
    pub authorized: bool,
    pub processing_ready: bool,
    pub free_access_available: bool,
}

impl AccessPresentation {
    pub fn project(profile: Option<&Profile>, client_terms: u32, signed_in: bool,
        fresh: bool, storage_available: bool) -> Self {
        let authority = Authority::evaluate(profile, client_terms, signed_in && fresh, storage_available);
        let terms_match = authority.terms_relation == TermsRelation::Match;
        let terms_action_available = signed_in && fresh && storage_available && terms_match;
        let state = if !signed_in { "signInRequired" }
            else if !authority.entitlement_active { "entitlementRequired" }
            else if !terms_match { "termsVersionMismatch" }
            else if !authority.consent_accepted { "onlineConsentRequired" }
            else if !authority.protocol_supported || !storage_available { "protocolUnavailable" }
            else { "ready" };
        let enable_allowed = !signed_in || storage_available &&
            !(authority.entitlement_active && !authority.protocol_supported ||
                matches!(authority.terms_relation, TermsRelation::ServerOlder | TermsRelation::ServerNewer));
        Self { client_terms_version: client_terms, state, enable_allowed, terms_match,
            terms_action_available, consent_accepted: authority.consent_accepted,
            needs_terms_acceptance: terms_action_available && authority.entitlement_active && !authority.consent_accepted,
            entitlement_active: authority.entitlement_active, authorized: authority.authorized,
            display_entitlement_active: signed_in && profile.is_some_and(|p| p.wire()["styleAccess"]["state"] == "active"),
            processing_ready: authority.processing_ready,
            free_access_available: profile.is_some_and(|p|
                FreeAccessOffer::from_profile(p.wire(), signed_in && fresh && storage_available).available()) }
    }
}
