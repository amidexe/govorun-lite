use crate::{AccountAccessHost, AccountAccessService, AccountRequest, AccountSession, Authority,
    Error, Method, Profile, Result, TermsRelation};
use serde::Deserialize;
use serde_json::{json, Value};

/// The user's shown offer and explicit action, never a locally calculated quota.
#[derive(Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FreeAccessActivation {
    pub confirmed: bool,
    pub terms_version: Option<u32>,
    pub preferred_style: String,
    pub expected_program_version: Option<String>,
}

impl AccountAccessService {
    pub async fn activate(&self, host: &dyn AccountAccessHost, epoch: u64,
        command: FreeAccessActivation) -> Result<Profile> {
        if !command.confirmed || host.terms_version() == 0 ||
            command.terms_version.is_some_and(|v| v != host.terms_version()) ||
            command.preferred_style.is_empty() || command.preferred_style.len() > 64 ||
            command.preferred_style.bytes().any(|b| b.is_ascii_control()) {
            return Err(Error::InvalidRequest);
        }
        let _access = self.gate.lock().await;
        // Re-read even a fresh cache: offers may have changed since the UI opened.
        let profile = crate::consent::refresh(host, epoch).await?;
        let authority = Authority::evaluate(Some(&profile), host.terms_version(), true, true);
        if authority.terms_relation != TermsRelation::Match { return Err(Error::TermsMismatch); }
        if !authority.consent_accepted && command.terms_version.is_none() {
            return Err(Error::ConsentUnconfirmed);
        }
        let program = &profile.wire()["promoProgram"];
        if command.expected_program_version.as_deref().is_some_and(|shown|
            program["programVersion"].as_str() != Some(shown) || program["state"] == "disabled") {
            return Err(Error::InvalidRequest);
        }
        let offer = crate::FreeAccessOffer::from_profile(profile.wire(), true);
        let joining_program = command.expected_program_version.is_some() && offer.program_available;
        if authority.entitlement_active && !joining_program {
            // Login/repeated confirmation restores existing access, without issuing another grant.
            let profile = if authority.consent_accepted { profile } else {
                self.accept_locked(host, epoch, host.terms_version()).await?
            };
            return confirmed_access(profile, host.terms_version());
        }
        let request = activation_request(profile.wire(), &command)?;
        let response = host.request(epoch, request).await;
        host.current(epoch)?;
        let response = response?;
        if response.status == 409 {
            crate::consent::refresh(host, epoch).await?;
            return Err(Error::Http(409));
        }
        response.success()?;
        // A successful POST is not proof of usable access or persisted consent.
        let profile = crate::consent::refresh(host, epoch).await?;
        confirmed_access(profile, host.terms_version())
    }
}

fn confirmed_access(profile: Profile, terms: u32) -> Result<Profile> {
    let authority = Authority::evaluate(Some(&profile), terms, true, true);
    if authority.terms_relation != TermsRelation::Match { return Err(Error::TermsMismatch); }
    if !authority.consent_accepted { return Err(Error::ConsentUnconfirmed); }
    if !authority.authorized { return Err(Error::InvalidResponse); }
    Ok(profile)
}

fn activation_request(profile: &Value, command: &FreeAccessActivation) -> Result<AccountRequest> {
    let program = &profile["promoProgram"];
    let offer = crate::FreeAccessOffer::from_profile(profile, true);
    let (path, mut body) = if offer.program_visible {
        let campaign = program["campaignId"].as_str().filter(|s| !s.trim().is_empty());
        let version = program["programVersion"].as_str().filter(|s| !s.trim().is_empty());
        if !offer.program_available || version != command.expected_program_version.as_deref() {
            return Err(Error::InvalidRequest);
        }
        ("/v1/billing/promo-program", json!({"campaignId":campaign, "programVersion":version}))
    } else {
        // A vanished/closed Promo must never silently turn into a different offer.
        if command.expected_program_version.is_some() || !offer.starter_available {
            return Err(Error::InvalidRequest);
        }
        ("/v1/billing/starter-grant", json!({"preferredStyle":command.preferred_style}))
    };
    if let Some(version) = command.terms_version { body["termsVersion"] = json!(version); }
    AccountRequest::new(Method::Post, path, Some(body))
}

impl AccountSession {
    pub async fn activate_free_access(&self, epoch: u64, command: FreeAccessActivation) -> Result<Profile> {
        self.inner.access.activate(self, epoch, command).await
    }
}
