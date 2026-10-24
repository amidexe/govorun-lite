use crate::{AccountRequest, AccountSession, Credentials, Error, HttpResponse, Method, Profile, Result, StoredSession};
use crate::session::Lease;
use serde_json::json;

impl AccountSession {
    /// A 401 refresh is shared across all account consumers. Issuance is never
    /// automatically replayed: its journal owns recovery/idempotency semantics.
    pub async fn request(&self, request: AccountRequest) -> Result<HttpResponse> {
        self.request_for_session(self.snapshot().session_generation, request).await
    }

    /// A delayed caller must never borrow the replacement account's credentials.
    pub async fn request_for_session(&self, epoch: u64, request: AccountRequest) -> Result<HttpResponse> {
        self.current(epoch)?;
        // Explicit confirmation and the shown offer belong to AccountAccessService.
        if crate::AccountAccessService::owns(&request) { return Err(Error::InvalidRequest); }
        if request.method() == Method::Get && request.path() == "/v1/billing/catalog" {
            return self.catalog_at(epoch).await;
        }
        if request.method() == Method::Get {
            if let Some(id) = request.path().strip_prefix("/v1/billing/tbank/checkout/") {
                return self.reconcile_checkout(epoch, id).await;
            }
        }
        if request.method() == Method::Post && request.path() == "/v1/billing/tbank/preview" {
            return self.prepare_checkout(epoch,
                request.body().cloned().ok_or(Error::InvalidRequest)?).await;
        }
        if request.method() == Method::Post {
            if let Some(id) = request.path().strip_prefix("/v1/billing/tbank/checkout/")
                .and_then(|rest| rest.strip_suffix("/cancel-binding")) {
                if request.body().is_some() { return Err(Error::InvalidRequest); }
                return self.cancel_checkout_binding(epoch, id).await;
            }
        }
        if request.method() == Method::Post && (request.path() == "/v1/billing/tbank/checkout" ||
            request.path() == "/v1/billing/change" && request.body().is_some_and(|b| b.get("subscriptionConsentToken").is_some_and(|v| !v.is_null()))) {
            return self.confirm_checkout(epoch, request.path(),
                request.body().cloned().ok_or(Error::InvalidRequest)?).await;
        }
        let (sequence, revision) = {
            let mut state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            state.profile_request += 1;
            (state.profile_request, state.profile_revision)
        };
        let response = self.request_at(epoch, request.clone()).await?;
        if response.status == 429 && request.path() == "/v1/style-passes/issue"
            && crate::style_issue::quota_limit(&response.body).is_some() {
            self.publish_access(response.body["styleAccess"].clone(), epoch, sequence, revision).await?;
        }
        if (200..300).contains(&response.status) {
            if request.path() == "/v1/style-usage/activate" {
                self.publish_profile(Profile::try_from(response.body.clone())?, epoch, sequence).await?;
                self.changed();
            } else if response.body.get("styleAccess").is_some_and(serde_json::Value::is_object) {
                self.publish_access(response.body["styleAccess"].clone(), epoch, sequence, revision).await?;
            }
            let revoked_current_device = {
                let state = self.state();
                if state.epoch != epoch { return Err(Error::SessionChanged); }
                state.tokens.as_ref().is_some_and(|tokens|
                    request.path().strip_prefix("/v1/devices/") == Some(tokens.device_id.as_str()))
            };
            if request.method() == Method::Delete &&
                ((request.path() == "/v1/account" && response.body["status"] == "deleted") || revoked_current_device) {
                let (old, ended_epoch) = {
                    let mut state = self.mutate();
                    if state.epoch != epoch { return Err(Error::SessionChanged); }
                    let old = state.tokens.clone();
                    (old, self.end_locked(&mut state))
                };
                self.changed();
                self.complete_sign_out(old, ended_epoch).await?;
            }
        }
        Ok(response)
    }

    pub(crate) async fn request_at(&self, epoch: u64, request: AccountRequest) -> Result<HttpResponse> {
        if crate::BillingManagementCommand::owns(&request) { return Err(Error::InvalidRequest); }
        if crate::BillingStatusService::tracks(&request) {
            return self.inner.billing.request(self, epoch, request).await;
        }
        self.authorized_request_at(epoch, request).await
    }

    pub(crate) async fn authorized_request_at(&self, epoch: u64, request: AccountRequest) -> Result<HttpResponse> {
        let lease = self.lease()?;
        if lease.epoch != epoch { return Err(Error::SessionChanged); }
        let first = self.send_scoped(lease.epoch, lease.boundary.clone(), request.clone(),
            Some(lease.tokens.access_token.clone())).await?;
        if first.status != 401 || !request.may_retry_unauthorized() { return Ok(first); }
        let refreshed = self.refresh_credentials(lease).await?;
        let result = self.send_scoped(refreshed.epoch, refreshed.boundary, request,
            Some(refreshed.tokens.access_token)).await?;
        if result.status == 401 {
            self.expire_if_current(refreshed.epoch, refreshed.token_revision).await?;
        }
        Ok(result)
    }

    async fn refresh_credentials(&self, failed: Lease) -> Result<Lease> {
        let _refresh = self.inner.refresh_gate.lock().await;
        self.current(failed.epoch)?;
        let current = self.lease()?;
        // A restore of the same pair bumps the revision without rotating; only a
        // different access token means another caller already refreshed.
        if current.tokens.access_token != failed.tokens.access_token { return Ok(current); }
        let request = AccountRequest::internal(Method::Post, "/v1/auth/refresh",
            Some(json!({"refreshToken": current.tokens.refresh_token})));
        let response = self.send_scoped(current.epoch, current.boundary.clone(), request, None).await?;
        if response.status == 401 {
            self.expire_if_current(current.epoch, current.token_revision).await?;
            return Err(Error::Http(401));
        }
        let tokens: Credentials = serde_json::from_value(response.success()?.body)
            .map_err(|_| Error::InvalidResponse)?;
        tokens.validate().map_err(|_| Error::InvalidResponse)?;
        if !current.tokens.device_id.is_empty() && tokens.device_id != current.tokens.device_id {
            return Err(Error::InvalidResponse);
        }
        let _storage = self.inner.storage_gate.lock().await;
        self.current(current.epoch)?;
        let profile = self.state().profile.clone();
        let saved = self.inner.store.save(StoredSession { version: 1, tokens: tokens.clone(),
            last_confirmed_profile: profile }, false).await;
        {
            let mut state = self.mutate();
            if state.epoch != current.epoch { return Err(Error::SessionChanged); }
            // Refresh rotation already happened server-side. Keep the new pair
            // in memory if storage failed, close permission to use the network, and let an
            // explicit recovery persist it. Never replay the obsolete refresh.
            state.tokens = Some(tokens);
            state.token_revision += 1;
            state.storage_available = saved.is_ok();
            if saved.is_err() { state.profile_fresh = false; }
        }
        self.changed();
        saved?;
        self.lease()
    }

    async fn expire_if_current(&self, epoch: u64, revision: u64) -> Result<()> {
        let ended = {
            let mut state = self.mutate();
            if state.epoch != epoch || state.token_revision != revision { None } else {
                let ended = self.end_locked(&mut state);
                state.ended_by_server = Some(ended);
                Some(ended)
            }
        };
        if let Some(epoch) = ended { self.finish_logout(epoch).await?; }
        Ok(())
    }
}
