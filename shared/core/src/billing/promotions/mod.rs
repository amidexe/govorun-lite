mod code;
mod controller;
mod offer;
mod service;
pub use access::{PromotionAccess, PromotionAccessKind};
pub(crate) use code::normalize as normalize_code;
pub(crate) use controller::failure_message;
pub use controller::{PromotionController, PromotionEffect};
pub(crate) use offer::uuid as valid_preview_id;
pub(crate) use offer::{inspect as inspect_offer, validate as validate_offer};
pub use service::{PromotionHost, PromotionService};

use crate::{AccountRequest, AccountSession, Error, Future, HttpResponse, Profile, Result};

impl PromotionHost for AccountSession {
    fn terms_version(&self) -> u32 {
        self.inner.terms
    }
    fn current(&self, epoch: u64) -> Result<()> {
        // Identity fencing must not discard a confirmed grant when persistence fails.
        // Each network request still requires the storage-available session lease.
        let state = self.state();
        if state.epoch != epoch {
            return Err(Error::SessionChanged);
        }
        if state.tokens.is_none() {
            return Err(Error::SignedOut);
        }
        Ok(())
    }
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        Box::pin(self.request_at(epoch, request))
    }
    fn refresh(&self, epoch: u64) -> Future<'_, Result<Profile>> {
        Box::pin(self.refresh_profile_at(epoch))
    }
}

impl AccountSession {
    pub async fn preview_promotion(
        &self,
        epoch: u64,
        code: &str,
        requested_offer: Option<&str>,
    ) -> Result<HttpResponse> {
        self.inner
            .promotions
            .preview_promotion(self, epoch, code, requested_offer)
            .await
    }
    pub async fn confirm_promotion(
        &self,
        epoch: u64,
        preview_id: &str,
        confirmed: bool,
    ) -> Result<PromotionEffect> {
        self.inner
            .promotions
            .confirm_promotion(self, epoch, preview_id, confirmed)
            .await
    }
    pub async fn redeem_promotion(
        &self,
        epoch: u64,
        preview_id: &str,
        confirmed: bool,
    ) -> Result<HttpResponse> {
        self.inner
            .promotions
            .redeem_promotion(self, epoch, preview_id, confirmed)
            .await
    }
}
mod access;
