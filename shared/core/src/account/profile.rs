use crate::{AccountSession, Error, Profile, Result, StoredSession};

impl AccountSession {
    pub async fn refresh_profile(&self) -> Result<Profile> {
        let epoch = self.state().epoch;
        self.refresh_profile_at(epoch).await
    }

    pub(crate) async fn refresh_profile_at(&self, epoch: u64) -> Result<Profile> {
        let result = crate::refresh_account_profile(self, epoch).await;
        if matches!(result, Err(Error::Http(403) | Error::TermsMismatch | Error::ConsentUnconfirmed)) {
            self.clear_confirmed_style_profile(epoch).await?;
        }
        result
    }

    async fn clear_confirmed_style_profile(&self, epoch: u64) -> Result<()> {
        let _storage = self.inner.storage_gate.lock().await;
        self.current(epoch)?;
        let tokens = {
            let mut state = self.mutate();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            // A newer successful publication supersedes a stale rejection.
            if state.style_profile_verified || state.profile.is_none() { return Ok(()); }
            state.profile = None;
            state.style_profile_verified = false;
            state.profile_revision += 1;
            state.tokens.clone().ok_or(Error::SignedOut)?
        };
        let saved = self.inner.store.save(StoredSession { version: 1, tokens,
            last_confirmed_profile: None }, false).await;
        {
            let mut state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            state.storage_available = saved.is_ok();
        }
        self.changed();
        saved
    }

    pub(crate) async fn publish_profile(&self, profile: Profile, epoch: u64, sequence: u64) -> Result<Profile> {
        self.commit_profile(profile, epoch, sequence, None).await
    }

    async fn commit_profile(&self, profile: Profile, epoch: u64, sequence: u64,
        expected_revision: Option<u64>) -> Result<Profile> {
        let _storage = self.inner.storage_gate.lock().await;
        let mut profile = profile;
        let tokens = {
            let mut state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            if expected_revision.is_some_and(|revision| revision != state.profile_revision) {
                return state.profile.clone().ok_or(Error::InvalidResponse);
            }
            if sequence < state.profile_applied { return state.profile.clone().ok_or(Error::InvalidResponse); }
            if let Some(old) = &state.profile {
                let confirmed = &old.wire()["styleAccess"]["usageBalance"];
                if crate::usage_ledger::balance_binding::older(&profile.wire()["styleAccess"]["usageBalance"], confirmed) {
                    let mut wire = profile.wire().clone();
                    wire["styleAccess"]["usageBalance"] = confirmed.clone();
                    profile = Profile::try_from(wire)?;
                }
            }
            if state.profile_fresh && state.profile.as_ref().is_some_and(|old| old.wire() == profile.wire()) {
                state.profile_applied = sequence;
                state.activity.profile_published(epoch, &profile, expected_revision.is_none());
                return Ok(profile);
            }
            let tokens = state.tokens.clone().ok_or(Error::SignedOut)?;
            if (!tokens.device_id.is_empty() && tokens.device_id != profile.device_id()) ||
                state.profile.as_ref().is_some_and(|old| old.id() != profile.id()) {
                return Err(Error::InvalidResponse);
            }
            tokens
        };
        let saved = self.inner.store.save(StoredSession { version: 1, tokens,
            last_confirmed_profile: Some(profile.clone()) }, false).await;
        let changed = {
            let mut state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            let changed = saved.is_ok() || state.storage_available != saved.is_ok();
            if changed { state.revision += 1; }
            state.storage_available = saved.is_ok();
            if saved.is_ok() {
                state.profile = Some(profile.clone());
                state.profile_applied = sequence;
                state.profile_revision += 1;
                state.profile_fresh = true;
                state.style_profile_verified = true;
                state.activity.profile_published(epoch, &profile, expected_revision.is_none());
            }
            changed
        };
        if changed { self.changed(); }
        saved?;
        Ok(profile)
    }

    pub(crate) async fn publish_access(&self, access: serde_json::Value, epoch: u64, sequence: u64,
        captured_revision: u64) -> Result<()> {
        let profile = {
            let state = self.state();
            if state.epoch != epoch { return Err(Error::SessionChanged); }
            if state.profile_revision != captured_revision { return Ok(()); }
            let Some(profile) = &state.profile else { return Ok(()); };
            let old = &profile.wire()["styleAccess"]["usageBalance"];
            let incoming = &access["usageBalance"];
            if crate::usage_ledger::balance_binding::older(incoming, old) { return Ok(()); }
            let mut value = profile.wire().clone();
            value["styleAccess"] = access;
            Profile::try_from(value)?
        };
        self.commit_profile(profile, epoch, sequence, Some(captured_revision)).await?;
        Ok(())
    }
}

impl crate::ProfileRefreshHost for AccountSession {
    fn begin_profile_refresh(&self, epoch: u64) -> Result<crate::ProfileUpdate> {
        let mut state = self.state();
        if state.epoch != epoch { return Err(Error::SessionChanged); }
        let tokens = state.tokens.as_ref().ok_or(Error::SignedOut)?;
        let identity = crate::ProfileIdentity {
            account_id: state.profile.as_ref().map(|profile| profile.id().to_owned()),
            device_id: Some(tokens.device_id.clone()).filter(|id| !id.is_empty()),
        };
        state.profile_request += 1;
        Ok(crate::ProfileUpdate { identity, revision: state.profile_request })
    }

    fn publish_refreshed_profile(&self, epoch: u64, update: crate::ProfileUpdate,
        profile: Profile) -> crate::Future<'_, Result<Profile>> {
        Box::pin(self.publish_profile(profile, epoch, update.revision))
    }

    fn profile_unavailable(&self, epoch: u64, update: &crate::ProfileUpdate,
        _projection: crate::ProfileUnavailable, cause: Error) {
        let changed = {
            let mut state = self.state();
            if state.epoch != epoch || update.revision < state.profile_applied { return; }
            state.activity.observe(epoch, Err(cause));
            let revoke_style = matches!(cause, Error::SignedOut | Error::SessionChanged | Error::StorageUnavailable
                | Error::StorageInvalid | Error::TermsMismatch | Error::ConsentUnconfirmed
                | Error::Http(401 | 403));
            let changed = state.profile_fresh || revoke_style && state.style_profile_verified;
            if changed { state.revision += 1; state.profile_revision += 1; }
            state.profile_fresh = false;
            // A failed background observation does not revoke already confirmed
            // style authority. Explicit authority/storage failures still close it.
            if revoke_style {
                state.style_profile_verified = false;
            }
            state.profile_applied = update.revision;
            changed
        };
        if changed { self.changed(); }
    }
}
