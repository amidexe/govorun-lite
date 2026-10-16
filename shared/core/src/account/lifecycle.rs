use crate::{AccountRequest, AccountSession, Credentials, Error, Method, Result, StorageRead, StoredSession};
use crate::session::State;
use serde_json::{json, Value};

enum SignInSource { Exchange(Value), Refresh(String) }

impl AccountSession {
    pub fn cancel_sign_in(&self) {
        self.cancel_sign_in_generation(None);
    }

    pub(crate) fn cancel_sign_in_generation(&self, expected_epoch: Option<u64>) {
        let mut state = self.mutate();
        if expected_epoch.is_some_and(|epoch| state.epoch != epoch) { return; }
        if state.tokens.is_some() || state.logout_pending { return; }
        state.epoch += 1;
        state.signing_in = false;
        state.pending_device_limit = None;
        // A started native write cannot be cancelled by dropping its future.
        // Block restoration/new publication until its serialized rollback ends.
        if state.sign_in_publishing.is_some() {
            state.storage_available = false;
            state.logout_pending = true;
        }
        self.inner.boundary.send_replace(state.epoch);
        drop(state);
        self.changed();
    }

    pub async fn restore(&self, allow_interaction: bool) -> Result<()> {
        if self.state().storage_restart_pending {
            return if allow_interaction { self.restart_storage().await } else { Err(Error::StorageUnavailable) };
        }
        let epoch = self.state().epoch;
        if self.state().logout_pending {
            return if allow_interaction { self.finish_logout(epoch).await } else { Err(Error::StorageUnavailable) };
        }
        let _storage = self.inner.storage_gate.lock().await;
        self.current(epoch)?;
        if self.state().logout_pending { return Err(Error::StorageUnavailable); }
        // Failed rotation persistence must save the new memory credentials,
        // rather than reloading a now-invalid refresh token from storage.
        let memory = {
            let state = self.state();
            state.tokens.as_ref().map(|tokens| StoredSession { version: 1, tokens: tokens.clone(),
                last_confirmed_profile: state.profile.clone() })
        };
        let restored = if let Some(memory) = memory {
            self.inner.store.save(memory.clone(), allow_interaction).await.map(|()| StorageRead::Session(memory))
        } else { self.inner.store.load(allow_interaction).await };
        {
            let mut state = self.mutate();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            state.storage_available = restored.is_ok();
            match restored {
                Ok(StorageRead::Missing) => { state.tokens = None; state.profile = None; }
                Ok(StorageRead::Session(stored)) => {
                    if stored.version != 1 || stored.tokens.validate().is_err() {
                        state.storage_available = false;
                        drop(state); self.changed();
                        return Err(Error::StorageInvalid);
                    }
                    if let Some(profile) = &stored.last_confirmed_profile {
                        if !stored.tokens.device_id.is_empty() && profile.device_id() != stored.tokens.device_id {
                            state.storage_available = false;
                            drop(state); self.changed();
                            return Err(Error::StorageInvalid);
                        }
                    }
                    if state.tokens.is_none() { self.inner.catalog.identity_changed(epoch); self.inner.billing.identity_changed(epoch); self.inner.checkout.identity_changed(epoch); }
                    state.tokens = Some(stored.tokens);
                    state.profile = stored.last_confirmed_profile;
                    state.token_revision += 1;
                }
                Err(error) => { drop(state); self.changed(); return Err(error); }
            }
            state.profile_fresh = false;
            state.style_profile_verified = state.profile.is_some();
        }
        self.changed();
        Ok(())
    }

    pub async fn exchange(&self, body: Value) -> Result<()> {
        self.exchange_for_generation(self.snapshot().session_generation, body).await
    }

    pub async fn exchange_for_generation(&self, expected_epoch: u64, body: Value) -> Result<()> {
        crate::validate_sign_in_request(&body)?;
        self.provisional_sign_in(expected_epoch, SignInSource::Exchange(body)).await
    }

    pub(crate) async fn import_refresh_for_generation(&self, expected_epoch: u64, token: String) -> Result<()> {
        crate::sign_in::validate_refresh_import(&token)?;
        self.provisional_sign_in(expected_epoch, SignInSource::Refresh(token)).await
    }

    async fn provisional_sign_in(&self, expected_epoch: u64, source: SignInSource) -> Result<()> {
        let (epoch, boundary) = {
            let mut state = self.mutate();
            if state.epoch != expected_epoch { return Err(Error::SessionChanged); }
            if state.tokens.is_some() || state.signing_in { return Err(Error::InvalidRequest); }
            if state.logout_pending || !state.storage_available { return Err(Error::StorageUnavailable); }
            state.epoch += 1;
            state.signing_in = true;
            self.inner.boundary.send_replace(state.epoch);
            (state.epoch, self.inner.boundary.subscribe())
        };
        let mut attempt = crate::sign_in_commit::SignInAttempt::new(self.clone(), epoch);
        self.changed();
        let host = crate::session_sign_in::SessionSignIn { session: self, boundary };
        let result = match source {
            SignInSource::Exchange(body) => crate::complete_sign_in(&host, epoch, body).await,
            SignInSource::Refresh(token) => crate::sign_in::complete_refresh_import(&host, epoch, token).await,
        };
        attempt.finish();
        result
    }

    pub async fn sign_out(&self) -> Result<()> {
        let (old, epoch) = self.start_sign_out();
        self.complete_sign_out(old, epoch).await
    }

    pub(crate) async fn restart_storage(&self) -> Result<()> {
        // Revoke in-flight work immediately; publish only after the namespace
        // switch, so native logout consumers cannot clear the previous vault.
        let epoch = {
            let mut state = self.mutate();
            let epoch = self.end_locked(&mut state);
            state.storage_available = false;
            state.storage_restart_pending = true;
            epoch
        };
        let _storage = self.inner.storage_gate.lock().await;
        self.current(epoch)?;
        let result = self.inner.store.restart().await;
        {
            let mut state = self.mutate();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            state.storage_available = result.is_ok();
            state.logout_pending = result.is_err();
            state.storage_restart_pending = result.is_err();
        }
        self.changed();
        result
    }

    pub(crate) fn start_sign_out(&self) -> (Option<Credentials>, u64) {
        let result = {
            let mut state = self.mutate();
            let old = state.tokens.clone();
            let epoch = self.end_locked(&mut state);
            (old, epoch)
        };
        self.changed();
        result
    }

    pub(crate) async fn complete_sign_out(&self, old: Option<Credentials>, epoch: u64) -> Result<()> {
        // The local fence completes even when remote revocation is unreachable.
        let result = self.finish_logout(epoch).await;
        if let Some(old) = old {
            let transport = self.inner.transport.clone();
            tokio::spawn(async move {
                let _ = transport.send(AccountRequest::internal(Method::Post, "/v1/auth/logout",
                    Some(json!({"refreshToken": old.refresh_token}))), None).await;
            });
        }
        result
    }

    pub(crate) fn end_locked(&self, state: &mut State) -> u64 {
        state.epoch += 1;
        state.devices = Default::default();
        self.inner.checkout.identity_changed(state.epoch);
        self.inner.billing.identity_changed(state.epoch);
        state.tokens = None;
        state.profile = None;
        state.profile_fresh = false;
        state.style_profile_verified = false;
        state.signing_in = false;
        state.pending_device_limit = None;
        state.logout_pending = true;
        state.profile_revision += 1;
        self.inner.boundary.send_replace(state.epoch);
        state.epoch
    }

    pub(crate) async fn finish_logout(&self, epoch: u64) -> Result<()> {
        self.changed();
        let _storage = self.inner.storage_gate.lock().await;
        self.current(epoch)?;
        let result = self.inner.store.end().await;
        {
            let mut state = self.mutate();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            state.logout_pending = result.is_err();
            state.storage_available = result.is_ok();
        }
        self.changed();
        result
    }
}
