use crate::{AccountAccessHost, AccountAccessService, AccountRequest, AccountSession, Authority, Error, Method, Profile, Result, TermsRelation};
use serde_json::json;

impl AccountAccessService {
    /// All clients use this operation with the exact terms shown to the user.
    /// Serialization coalesces repeated acceptance through the confirmed profile.
    pub async fn accept(&self, host: &dyn AccountAccessHost, epoch: u64, version: u32,
        confirmed: bool) -> Result<Profile> {
        if !confirmed || version == 0 || version != host.terms_version() { return Err(Error::InvalidRequest); }
        let _consent = self.gate.lock().await;
        self.accept_locked(host, epoch, version).await
    }

    pub(crate) async fn accept_locked(&self, host: &dyn AccountAccessHost, epoch: u64, version: u32) -> Result<Profile> {
        host.current(epoch)?;
        let profile = match host.fresh_access_profile(epoch)? {
            Some(profile) => profile,
            None => refresh(host, epoch).await?,
        };
        host.current(epoch)?;
        let authority = Authority::evaluate(Some(&profile), version, true, true);
        if authority.terms_relation != TermsRelation::Match { return Err(Error::TermsMismatch); }
        if authority.consent_accepted { return Ok(profile); }
        let response = host.request(epoch, AccountRequest::new(Method::Post,
            "/v1/consents/online-stylization", Some(json!({"version": version})))?).await;
        host.current(epoch)?;
        let response = response?;
        if response.status == 409 {
            refresh(host, epoch).await?;
            return Err(Error::TermsMismatch);
        }
        response.success()?;
        let profile = refresh(host, epoch).await?;
        if !Authority::evaluate(Some(&profile), version, true, true).consent_accepted {
            return Err(Error::ConsentUnconfirmed);
        }
        Ok(profile)
    }
}

pub(crate) async fn refresh(host: &dyn AccountAccessHost, epoch: u64) -> Result<Profile> {
    host.current(epoch)?;
    let result = host.refresh_access_profile(epoch).await;
    host.current(epoch)?;
    result
}

impl AccountSession {
    pub async fn accept_online_terms(&self, epoch: u64, version: u32, confirmed: bool) -> Result<Profile> {
        self.inner.access.accept(self, epoch, version, confirmed).await
    }
}
