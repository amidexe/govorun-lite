mod attempt;
mod cancellation;
mod confirmation;
mod completion;
mod discovery;
mod failure;
mod journal;
mod interaction;
mod observed;
mod lifecycle;
mod plan_change;
mod preparation;
mod presentation;
mod preview;
mod preview_action;
mod recovery;
mod recovery_failure;
mod session;

pub use attempt::CheckoutService;
pub use interaction::CheckoutInteraction;
pub use failure::CheckoutFailure;
pub use lifecycle::CheckoutSnapshot;
pub use recovery_failure::CheckoutRecoveryFailure;
pub(crate) use preparation::same_published_catalog;
pub use preparation::{prepare_checkout, CheckoutHost};
pub(crate) use presentation::projection;
pub use preview::validate_preview;
pub(crate) use preview_action::project_preview;

use crate::{AccountRequest, AccountSession, CatalogChannel, Future, HttpResponse, ProfileUpdate, Result};
use serde_json::Value;

impl CheckoutHost for AccountSession {
    fn checkout_changed(&self, snapshot: &CheckoutSnapshot) -> Result<()> {
        self.current(snapshot.epoch)?;
        self.publish_checkout_state(snapshot.epoch);
        Ok(())
    }
    fn begin_checkout_profile(&self, epoch: u64) -> Result<ProfileUpdate> {
        self.current(epoch)?;
        let mut state = self.state();
        if state.epoch != epoch { return Err(crate::Error::SessionChanged); }
        let profile = state.profile.as_ref().ok_or(crate::Error::SignedOut)?;
        let identity = crate::ProfileIdentity { account_id: Some(profile.id().into()),
            device_id: Some(profile.device_id().into()) };
        state.profile_request += 1;
        Ok(ProfileUpdate { identity, revision: state.profile_request })
    }
    fn publish_checkout_profile(&self, epoch: u64, update: ProfileUpdate,
        profile: crate::Profile) -> Future<'_, Result<()>> {
        Box::pin(async move {
            self.publish_profile(profile, epoch, update.revision).await?;
            self.current(epoch)?;
            let fresh = { let state = self.state();
                state.profile_fresh && state.storage_available && state.profile_applied >= update.revision };
            self.changed();
            if fresh { Ok(()) } else { Err(crate::Error::StorageUnavailable) }
        })
    }
    fn checkout_account(&self, epoch: u64) -> Result<String> {
        self.current(epoch)?;
        self.profile()
            .map(|p| p.id().to_owned())
            .ok_or(crate::Error::StorageUnavailable)
    }
    fn load_checkout(&self, epoch: u64, slot: String) -> Future<'_, Result<Option<Value>>> {
        Box::pin(async move {
            let _storage = self.inner.storage_gate.lock().await;
            self.current(epoch)?;
            let result = self.inner.store.load_checkout(slot).await?;
            self.current(epoch)?;
            Ok(result)
        })
    }
    fn save_checkout(
        &self,
        epoch: u64,
        slot: String,
        record: Option<Value>,
    ) -> Future<'_, Result<()>> {
        Box::pin(async move {
            let _storage = self.inner.storage_gate.lock().await;
            self.current(epoch)?;
            self.inner.store.save_checkout(slot, record).await?;
            self.current(epoch)
        })
    }
    fn current(&self, epoch: u64) -> Result<()> {
        self.current(epoch)?;
        self.lease().map(|_| ())
    }

    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        if request.method() == crate::Method::Get && request.path() == "/v1/billing/catalog" {
            return Box::pin(self.refresh_catalog_response(epoch));
        }
        // Only the confirmed CheckoutService may submit a plan change here;
        // public generic requests cannot bypass checkout or management admission.
        if request.method() == crate::Method::Post && request.path() == "/v1/billing/change" {
            return Box::pin(self.inner.billing.request(self, epoch, request));
        }
        Box::pin(self.request_at(epoch, request))
    }
}

impl AccountSession {
    pub async fn cancel_checkout_binding(&self, epoch: u64, id: &str) -> Result<HttpResponse> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner.checkout.cancel_binding(self, epoch, id).await
    }

    pub async fn reconcile_checkout(&self, epoch: u64, id: &str) -> Result<HttpResponse> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner.checkout.reconcile(self, epoch, id).await
    }

    pub async fn prepare_checkout(&self, epoch: u64, request: Value) -> Result<HttpResponse> {
        self.prepare_checkout_for_catalog(epoch, request, None, CatalogChannel::Direct).await
    }

    pub async fn prepare_checkout_for_catalog(&self, epoch: u64, request: Value,
        displayed_catalog: Option<&Value>, channel: CatalogChannel) -> Result<HttpResponse> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner
            .checkout
            .prepare(self, epoch, request, displayed_catalog, channel)
            .await
    }

    pub async fn confirm_checkout(
        &self,
        epoch: u64,
        path: &str,
        request: Value,
    ) -> Result<HttpResponse> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner
            .checkout
            .confirm(self, epoch, path, request)
            .await
    }

    pub async fn confirm_prepared_checkout(&self, epoch: u64, input: Value) -> Result<HttpResponse> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner.checkout.confirm_prepared(self, epoch, input).await
    }

    pub async fn begin_checkout_interaction(&self, epoch: u64, selection: Value,
        displayed: Option<&Value>, channel: CatalogChannel) -> Result<CheckoutInteraction> {
        let _publication = lifecycle::SessionPublication { session: self, epoch };
        self.inner.checkout.begin_interaction(self, epoch, selection, displayed, channel).await
    }

    fn publish_checkout_state(&self, epoch: u64) {
        let mut state = self.state();
        if state.epoch != epoch { return; }
        state.revision += 1;
        drop(state);
        self.changed();
    }
}

pub(crate) use preparation::CatalogPublication;
