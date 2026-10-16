use crate::{AccountSession, Error, Future, Profile, ProfileSource, Result};

/// Profile effects must publish durably in the captured session before returning.
/// A missing fresh profile requests a refresh; cached consent is never authority.
pub trait AccountAccessHost: ProfileSource {
    fn terms_version(&self) -> u32;
    fn fresh_access_profile(&self, epoch: u64) -> Result<Option<Profile>>;
    fn refresh_access_profile(&self, epoch: u64) -> Future<'_, Result<Profile>>;
}

#[derive(Default)]
pub struct AccountAccessService { pub(crate) gate: tokio::sync::Mutex<()> }

impl AccountAccessService {
    pub fn owns(request: &crate::AccountRequest) -> bool {
        matches!(request.path(), "/v1/consents/online-stylization" |
            "/v1/billing/starter-grant" | "/v1/billing/promo-program")
    }
}

impl AccountAccessHost for AccountSession {
    fn terms_version(&self) -> u32 { self.inner.terms }
    fn fresh_access_profile(&self, epoch: u64) -> Result<Option<Profile>> {
        self.current(epoch)?;
        let state = self.state();
        if state.epoch != epoch { return Err(Error::SessionChanged); }
        Ok(state.profile.clone().filter(|_| state.profile_fresh && state.storage_available))
    }
    fn refresh_access_profile(&self, epoch: u64) -> Future<'_, Result<Profile>> {
        Box::pin(self.refresh_profile_at(epoch))
    }
}
