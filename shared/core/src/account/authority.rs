use crate::Profile;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TermsRelation { Unknown, Match, ServerOlder, ServerNewer }

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Authority {
    pub terms_relation: TermsRelation,
    pub consent_accepted: bool,
    pub entitlement_active: bool,
    pub protocol_supported: bool,
    pub authorized: bool,
    pub processing_ready: bool,
}

impl Authority {
    pub fn evaluate(profile: Option<&Profile>, client_terms: u32, fresh: bool, storage_available: bool) -> Self {
        let wire = profile.map(Profile::wire);
        let server_terms = wire.and_then(|p| p["onlineTermsVersion"].as_u64());
        let terms_relation = match server_terms {
            None | Some(0) => TermsRelation::Unknown,
            Some(value) if value == u64::from(client_terms) => TermsRelation::Match,
            Some(value) if value < u64::from(client_terms) => TermsRelation::ServerOlder,
            _ => TermsRelation::ServerNewer,
        };
        // Cached/offline consent never grants permission to use the network. A fresh profile
        // restores the local presentation without submitting the same consent again.
        let consent_accepted = fresh && terms_relation == TermsRelation::Match && wire.is_some_and(|p|
            p["onlineConsent"]["active"] == true && p["onlineConsent"]["acceptedVersion"] == client_terms);
        let entitlement_active = fresh && wire.is_some_and(|p| p["styleAccess"]["state"] == "active");
        let protocol_supported = wire.is_some_and(|p| protocol_supported(&p["styleAccess"]));
        let authorized = storage_available && consent_accepted && entitlement_active && protocol_supported;
        // Remembered service health is a display observation, not admission.
        // Each authorized attempt obtains its own transport result.
        let processing_ready = authorized;
        Self { terms_relation, consent_accepted, entitlement_active, protocol_supported, authorized, processing_ready }
    }
}

pub(crate) fn protocol_supported(access: &serde_json::Value) -> bool {
    access["quotaUnit"] == "text_credit" && access["charactersPerCredit"] == crate::credit_limits::CHARACTERS_PER_CREDIT &&
        access["maximumCreditsPerRequest"] == crate::style_inventory::BATCH && access["stylizationProtocol"] == "v2_style_passes_json"
}

impl crate::AccountSession {
    /// keeps only previously confirmed style authority across background
    /// observation failures. Checkout and other account mutations remain fresh-only.
    pub(crate) fn style_authority(&self) -> Authority {
        let state = self.state();
        Authority::evaluate(state.profile.as_ref(), self.inner.terms,
            state.tokens.is_some() && state.style_profile_verified && !state.logout_pending,
            state.storage_available)
    }
    pub(crate) fn style_access_current(&self, now: u64) -> bool {
        access_current(self.state().profile.as_ref(), now)
    }
    /// The working plane observes one publication, never a mix of revisions.
    pub fn style_observation(&self, now: u64) -> StyleObservation {
        let state = self.state();
        StyleObservation {
            snapshot: self.snapshot_locked(&state), profile: state.profile.clone(),
            access_current: access_current(state.profile.as_ref(), now),
            usage_fresh: state.activity.style_usage_fresh(state.profile_fresh),
        }
    }
}
pub struct StyleObservation {
    pub snapshot: crate::SessionSnapshot,
    pub profile: Option<Profile>,
    pub access_current: bool,
    pub usage_fresh: bool,
}
fn access_current(profile: Option<&Profile>, now: u64) -> bool {
    let now = now.min(i64::MAX as u64) as i64;
    let access = profile.map(|p| &p.wire()["styleAccess"]);
    access.is_some_and(|a| [ ("startsAt", true), ("endsAt", false) ].iter().all(|(name,start)| {
        if a[*name].is_null() { return true; }
        a[*name].as_str().and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .is_some_and(|date| if *start { date.timestamp_millis() <= now }
                else { date.timestamp_millis() > now })
    }))
}
