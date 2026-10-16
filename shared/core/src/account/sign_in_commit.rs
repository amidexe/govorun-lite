use crate::{AccountSession, Credentials, Error, Profile, Result, StoredSession};

/// Dropping the outer exchange revokes only its own admission, before a retained
/// publication worker can commit. Completed/newer sessions are left alone.
pub(crate) struct SignInAttempt {
    session: AccountSession,
    epoch: u64,
    finished: bool,
}

impl SignInAttempt {
    pub fn new(session: AccountSession, epoch: u64) -> Self {
        Self {
            session,
            epoch,
            finished: false,
        }
    }

    pub fn finish(&mut self) {
        {
            let mut state = self.session.mutate();
            if state.epoch == self.epoch {
                state.signing_in = false;
            }
        }
        self.finished = true;
        self.session.changed();
    }
}

impl Drop for SignInAttempt {
    fn drop(&mut self) {
        if !self.finished {
            self.session.cancel_sign_in_generation(Some(self.epoch));
        }
    }
}

impl AccountSession {
    pub(crate) async fn finish_sign_in_cancellation(&self) -> Result<()> {
        let epoch = self.state().epoch;
        let _storage = self.inner.storage_gate.lock().await;
        self.current(epoch)?;
        if self.state().logout_pending {
            return Err(Error::StorageUnavailable);
        }
        Ok(())
    }

    pub(crate) async fn commit_sign_in(
        &self,
        epoch: u64,
        tokens: Credentials,
        profile: Profile,
    ) -> Result<()> {
        let _storage = self.inner.storage_gate.lock().await;
        {
            let mut state = self.mutate();
            if state.epoch != epoch {
                return Err(Error::SessionChanged);
            }
            state.sign_in_publishing = Some(epoch);
        }
        let saved = self
            .inner
            .store
            .save(
                StoredSession {
                    version: 1,
                    tokens: tokens.clone(),
                    last_confirmed_profile: Some(profile.clone()),
                },
                true,
            )
            .await;
        {
            let mut state = self.mutate();
            if state.epoch == epoch {
                state.sign_in_publishing = None;
                state.signing_in = false;
                state.storage_available = saved.is_ok();
                self.inner.catalog.identity_changed(epoch);
                self.inner.billing.identity_changed(epoch);
                self.inner.checkout.identity_changed(epoch);
                state.tokens = Some(tokens);
                state.token_revision += 1;
                if saved.is_ok() {
                    state.activity.profile_published(epoch, &profile, true);
                } else {
                    state.activity.observe(epoch, saved);
                }
                state.profile = Some(profile);
                state.profile_fresh = saved.is_ok();
                state.style_profile_verified = saved.is_ok();
                state.profile_revision += 1;
                state.pending_device_limit = None;
                drop(state);
                self.changed();
                return saved;
            }
        }
        // Save may have reached protected storage after cancellation. End it
        // before releasing the gate; a later restore must never read that pair.
        let rollback = self.inner.store.end().await;
        {
            let mut state = self.mutate();
            state.sign_in_publishing = None;
            if !state.storage_restart_pending {
                state.storage_available = rollback.is_ok();
                state.logout_pending = rollback.is_err();
            }
        }
        self.changed();
        Err(Error::SessionChanged)
    }
}
