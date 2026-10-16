use crate::{AccountSession, AuthAttempt, AuthConfig, AuthDevice, AuthProvider, AuthSession, AuthStorage, Error, Result};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Default)]
struct AttemptState { loaded: bool, attempt: Option<AuthAttempt> }

/// Reserved synchronously when an IPC frame arrives, before spawning its work.
/// This preserves the user's Begin/Cancel order independently of task scheduling.
#[derive(Clone, Copy)]
pub struct AuthIntent(u64);

/// PKCE, callback verification, cancellation and device replacement have the
/// same owner across clients. Native code only opens the returned
/// browser URL and adapts protected storage / installation identity.
pub struct AuthFlow<S: AuthSession = AccountSession> {
    session: S,
    config: AuthConfig,
    store: Arc<dyn AuthStorage>,
    state: AsyncMutex<AttemptState>,
    boundary: Mutex<()>,
    generation: AtomicU64,
}

impl<S: AuthSession> AuthFlow<S> {
    pub fn new(session: S, config: AuthConfig, store: Arc<dyn AuthStorage>) -> Self {
        Self { session, config, store, state: AsyncMutex::new(AttemptState::default()),
            boundary: Mutex::new(()), generation: AtomicU64::new(0) }
    }

    fn invalidate(&self) -> u64 {
        let _boundary = self.boundary.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.session.cancel_sign_in();
        generation
    }

    pub fn reserve_begin(&self) -> Result<AuthIntent> {
        let snapshot = self.session.auth_snapshot();
        if snapshot.signed_in { return Err(Error::InvalidRequest); }
        if snapshot.logout_pending { return Err(Error::StorageUnavailable); }
        Ok(AuthIntent(self.invalidate()))
    }

    pub fn reserve_cancel(&self) -> AuthIntent { AuthIntent(self.invalidate()) }
    pub fn pending_intent(&self) -> AuthIntent { AuthIntent(self.generation.load(Ordering::SeqCst)) }
    pub fn check_intent(&self, intent: AuthIntent) -> Result<()> { self.current(intent.0) }

    fn current(&self, generation: u64) -> Result<()> {
        if self.generation.load(Ordering::SeqCst) == generation { Ok(()) } else { Err(Error::SessionChanged) }
    }

    fn signed_out(&self) -> Result<()> {
        let snapshot = self.session.auth_snapshot();
        if snapshot.signed_in || snapshot.signing_in { return Err(Error::InvalidRequest); }
        if !snapshot.storage_available || snapshot.logout_pending { return Err(Error::StorageUnavailable); }
        Ok(())
    }

    pub async fn begin(&self, provider: AuthProvider) -> Result<String> {
        self.begin_reserved(self.reserve_begin()?, provider).await
    }

    pub async fn begin_reserved(&self, intent: AuthIntent, provider: AuthProvider) -> Result<String> {
        let generation = intent.0;
        let mut state = self.state.lock().await;
        self.current(generation)?;
        state.loaded = true;
        state.attempt = None;
        if !self.session.auth_snapshot().storage_available { self.session.restore_for_auth().await?; }
        self.current(generation)?;
        self.signed_out()?;
        let attempt = AuthAttempt::create(provider)?;
        let url = self.config.start_url(&attempt)?;
        self.store.save_attempt(Some(attempt.clone()), true).await?;
        self.current(generation)?;
        state.attempt = Some(attempt);
        Ok(url)
    }

    async fn restored(&self, state: &mut AttemptState) -> Result<AuthAttempt> {
        if !state.loaded {
            let attempt = self.store.load_attempt().await?;
            if let Some(attempt) = &attempt { attempt.validate()?; }
            state.attempt = attempt;
            state.loaded = true;
        }
        state.attempt.clone().ok_or(Error::InvalidRequest)
    }

    /// A browser return without a callback may discard only an unexchanged
    /// attempt. Device replacement and an in-flight publication remain owned
    /// by the explicit auth workflow, independent of screen lifecycle events.
    pub async fn abandon_browser_reserved(&self, intent: AuthIntent) -> Result<()> {
        let mut state = self.state.lock().await;
        self.current(intent.0)?;
        let snapshot = self.session.auth_snapshot();
        if snapshot.signed_in || snapshot.signing_in { return Ok(()); }
        if !state.loaded {
            state.attempt = self.store.load_attempt().await?;
            state.loaded = true;
        }
        if state.attempt.as_ref().is_some_and(|attempt| attempt.handoff.is_none()) {
            self.store.save_attempt(None, false).await?;
            self.current(intent.0)?;
            state.attempt = None;
        }
        Ok(())
    }

    pub async fn callback(&self, url: &str, device: AuthDevice) -> Result<()> {
        self.callback_reserved(self.pending_intent(), url, device).await
    }

    pub async fn callback_reserved(&self, intent: AuthIntent, url: &str, device: AuthDevice) -> Result<()> {
        let generation = intent.0;
        let (body, epoch) = {
            let mut state = self.state.lock().await;
            self.current(generation)?;
            self.signed_out()?;
            let epoch = self.session.auth_snapshot().session_generation;
            let mut attempt = self.restored(&mut state).await?;
            self.current(generation)?;
            attempt.handoff = Some(self.config.callback(&attempt, url)?);
            let body = device.exchange(&attempt, None)?;
            self.store.save_attempt(Some(attempt.clone()), false).await?;
            self.current(generation)?;
            state.attempt = Some(attempt);
            (body, epoch)
        };
        self.current(generation)?;
        let result = self.session.exchange_auth(epoch, body).await;
        self.finish(generation, &result).await;
        result
    }

    pub async fn replace(&self, id: &str, confirmed: bool, device: AuthDevice) -> Result<()> {
        self.replace_reserved(self.pending_intent(), id, confirmed, device).await
    }

    pub async fn replace_reserved(&self, intent: AuthIntent, id: &str, confirmed: bool, device: AuthDevice) -> Result<()> {
        if !confirmed { return Err(Error::InvalidRequest); }
        let generation = intent.0;
        let (body, epoch) = {
            let mut state = self.state.lock().await;
            self.current(generation)?;
            self.signed_out()?;
            let projection = self.session.pending_device_limit().unwrap_or_default();
            let offered = projection["devices"].as_array()
                .is_some_and(|devices| devices.iter().any(|device| device["id"].as_str() == Some(id)));
            if !offered { return Err(Error::InvalidRequest); }
            let epoch = self.session.auth_snapshot().session_generation;
            let attempt = self.restored(&mut state).await?;
            self.current(generation)?;
            (device.exchange(&attempt, Some(id))?, epoch)
        };
        self.current(generation)?;
        let result = self.session.exchange_auth(epoch, body).await;
        self.finish(generation, &result).await;
        result
    }

    async fn finish(&self, generation: u64, result: &Result<()>) {
        if *result == Err(Error::Http(409)) && self.session.pending_device_limit().is_some_and(|value| value.is_object()) { return; }
        let mut state = self.state.lock().await;
        if self.current(generation).is_err() { return; }
        state.loaded = true;
        state.attempt = None;
        // A successfully persisted session stays valid if cleanup is denied.
        // Old handoffs cannot run while signed in; the native logout fence must
        // also cancel any persisted attempt before reporting durable logout.
        let _ = self.store.save_attempt(None, false).await;
    }

    pub async fn cancel(&self) -> Result<()> {
        // Invalidate network work before waiting for a storage write.
        self.cancel_reserved(self.reserve_cancel()).await
    }

    pub async fn cancel_reserved(&self, intent: AuthIntent) -> Result<()> {
        let generation = intent.0;
        let mut state = self.state.lock().await;
        self.current(generation)?;
        self.session.finish_auth_cancellation().await?;
        self.current(generation)?;
        state.loaded = true;
        state.attempt = None;
        self.store.save_attempt(None, false).await
    }

}

impl AuthFlow<AccountSession> {
    pub fn reserve_refresh_import(&self, token: &str) -> Result<AuthIntent> {
        if !self.config.refresh_import { return Err(Error::InvalidRequest); }
        crate::sign_in::validate_refresh_import(token)?;
        self.reserve_begin()
    }

    pub async fn import_refresh_reserved(&self, intent: AuthIntent, token: String) -> Result<()> {
        if !self.config.refresh_import { return Err(Error::InvalidRequest); }
        crate::sign_in::validate_refresh_import(&token)?;
        let epoch = {
            let mut state = self.state.lock().await;
            self.check_intent(intent)?;
            self.signed_out()?;
            if !state.loaded {
                state.attempt = self.store.load_attempt().await?;
                state.loaded = true;
            }
            self.check_intent(intent)?;
            if state.attempt.is_some() { return Err(Error::InvalidRequest); }
            self.session.snapshot().session_generation
        };
        self.check_intent(intent)?;
        let result = self.session.import_refresh_for_generation(epoch, token).await;
        self.finish(intent.0, &result).await;
        result
    }

    pub async fn logout_reserved(&self, intent: AuthIntent) -> Result<()> {
        self.check_intent(intent)?;
        // Close authority before waiting, then serialize the durable logout
        // behind any pending OAuth write so an old write cannot revive it.
        let (old, epoch) = self.session.start_sign_out();
        let mut state = self.state.lock().await;
        let result = self.session.complete_sign_out(old, epoch).await;
        state.loaded = true;
        state.attempt = None;
        result
    }

    pub async fn restart_storage_reserved(&self, intent: AuthIntent) -> Result<()> {
        self.check_intent(intent)?;
        let mut state = self.state.lock().await;
        self.check_intent(intent)?;
        let result = self.session.restart_storage().await;
        state.loaded = result.is_ok();
        state.attempt = None;
        result
    }
}

/// Destination of an existing Android login handoff; clocks/storage are port facts.
pub(crate) fn handoff_destination(subscription_at:i64,stylization_at:i64,now:i64)->&'static str {
    let fresh=|created:i64|created>0 && now.checked_sub(created).is_some_and(|age|(0..=600_000).contains(&age));
    if fresh(stylization_at) {"STYLIZATION"} else if fresh(subscription_at) {"SUBSCRIPTION"} else {"HOME"}
}
