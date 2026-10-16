use crate::{Authority, Credentials, Error, HttpResponse, Profile, Result, Storage, Transport};
use serde::Serialize;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::{watch, Mutex as AsyncMutex};

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub revision: u64,
    pub session_generation: u64,
    pub profile_revision: u64,
    pub signed_in: bool,
    pub signing_in: bool,
    pub profile_fresh: bool,
    pub storage_available: bool,
    pub logout_pending: bool,
    pub display_name: Option<String>,
    pub authority: Authority,
    pub access: crate::AccessPresentation,
}

/// Runtime entropy is a port: ordinary sessions use the OS source; controlled
/// runtimes can supply their own stream without changing scheduling rules.
pub struct SessionOptions {
    pub terms: u32,
    pub entropy: Arc<dyn Fn() -> u64 + Send + Sync>,
}
impl From<u32> for SessionOptions {
    fn from(terms:u32)->Self {
        Self {terms,entropy:Arc::new(||getrandom::u64().unwrap_or(u64::MAX/2))}
    }
}

pub(crate) struct State {
    pub activity: crate::activity::ActivityRefresh,
    pub revision: u64,
    pub epoch: u64,
    pub token_revision: u64,
    pub tokens: Option<Credentials>,
    pub profile: Option<Profile>,
    pub profile_fresh: bool,
    /// Last server-confirmed profile, including durable restoration for Styles.
    /// New account actions still require profile_fresh.
    pub style_profile_verified: bool,
    pub storage_available: bool,
    pub signing_in: bool,
    pub logout_pending: bool,
    pub sign_in_publishing: Option<u64>,
    pub profile_request: u64,
    pub storage_restart_pending: bool,
    pub profile_applied: u64,
    pub profile_revision: u64,
    pub pending_device_limit: Option<serde_json::Value>,
    pub devices: crate::devices::Devices,
}

impl State {
    fn new(entropy: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        Self { activity: crate::activity::ActivityRefresh::new(entropy), revision: 1, epoch: 1, token_revision: 0, tokens: None, profile: None,
            profile_fresh: false, style_profile_verified: false, storage_available: false, signing_in: false,
            logout_pending: false, sign_in_publishing: None, storage_restart_pending: false, profile_request: 0, profile_applied: 0, profile_revision: 0,
            pending_device_limit: None, devices: Default::default() }
    }
}

pub(crate) struct Inner {
    pub state: Mutex<State>,
    pub store: Arc<dyn Storage>,
    pub transport: Arc<dyn Transport>,
    pub storage_gate: AsyncMutex<()>,
    pub refresh_gate: AsyncMutex<()>,
    pub access: crate::AccountAccessService,
    pub promotions: crate::promotions::PromotionService,
    pub promotion_window: crate::promotions::PromotionController,
    pub catalog: crate::CatalogRefresh,
    pub billing: crate::BillingStatusService,
    pub checkout: crate::checkout::CheckoutService,
    pub subscription: crate::subscription::SubscriptionController,
    pub boundary: watch::Sender<u64>,
    pub changed: watch::Sender<u64>,
    pub terms: u32,
    pub entropy: Arc<dyn Fn() -> u64 + Send + Sync>,
}

/// Single account authority for both desktop platforms. Native services adapt
/// storage and consume snapshots; they do not own a second token-refresh loop.
#[derive(Clone)]
pub struct AccountSession { pub(crate) inner: Arc<Inner> }

pub(crate) struct Lease {
    pub epoch: u64,
    pub token_revision: u64,
    pub tokens: Credentials,
    pub boundary: watch::Receiver<u64>,
}

impl AccountSession {
    pub fn new(store: Arc<dyn Storage>, transport: Arc<dyn Transport>, options: impl Into<SessionOptions>) -> Result<Self> {
        let SessionOptions { terms, entropy }=options.into();
        if terms == 0 { return Err(Error::InvalidRequest); }
        let (boundary, _) = watch::channel(1);
        let (changed, _) = watch::channel(0);
        Ok(Self { inner: Arc::new(Inner { state: Mutex::new(State::new(entropy.clone())), store, transport,
            storage_gate: AsyncMutex::new(()), refresh_gate: AsyncMutex::new(()),
            access: Default::default(), promotions: Default::default(), promotion_window: Default::default(), catalog: Default::default(),
            checkout: Default::default(), billing: Default::default(), subscription: Default::default(),
            boundary, changed, terms, entropy }) })
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let state = self.state();
        self.snapshot_locked(&state)
    }

    pub(crate) fn snapshot_locked(&self, state: &State) -> SessionSnapshot {
        let mut access = crate::AccessPresentation::project(state.profile.as_ref(), self.inner.terms,
            state.tokens.is_some(), state.profile_fresh, state.storage_available);
        let style = Authority::evaluate(state.profile.as_ref(), self.inner.terms,
            state.tokens.is_some() && state.style_profile_verified && !state.logout_pending,
            state.storage_available);
        access.entitlement_active = style.entitlement_active;
        access.consent_accepted = style.consent_accepted;
        access.authorized = style.authorized;
        access.processing_ready = style.processing_ready;
        if style.processing_ready { access.state = "ready"; }
        SessionSnapshot { revision: state.revision, session_generation: state.epoch, profile_revision: state.profile_revision,
            signed_in: state.tokens.is_some(), signing_in: state.signing_in,
            profile_fresh: state.profile_fresh, storage_available: state.storage_available,
            logout_pending: state.logout_pending,
            display_name: state.profile.as_ref().map(|p| p.display_name().to_owned())
                .or_else(|| state.tokens.as_ref().map(|tokens| tokens.display_name.clone())),
            authority: Authority::evaluate(state.profile.as_ref(), self.inner.terms,
                state.tokens.is_some() && state.profile_fresh, state.storage_available),
            access }
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> { self.inner.changed.subscribe() }
    /// Host effect lifetimes observe the same fence as authorized transport.
    pub fn subscribe_boundary(&self) -> watch::Receiver<u64> { self.inner.boundary.subscribe() }

    /// Trusted native view only; never forward as a general UI event.
    pub fn profile(&self) -> Option<Profile> { self.state().profile.clone() }

    /// Native compatibility DTO: the strings are unusable as credentials. All
    /// authenticated HTTP must return to this core, which owns the real pair.
    pub fn native_projection(&self) -> serde_json::Value {
        let state = self.state();
        let snapshot = self.snapshot_locked(&state);
        let tokens = state.tokens.as_ref().map(|tokens| serde_json::json!({
            "accessToken": "host-owned-session", "refreshToken": "host-owned-session",
            "displayName": tokens.display_name, "deviceId": tokens.device_id, "deviceLimit": tokens.device_limit,
        }));
        let profile = state.profile.as_ref().map(|p| {
            let mut wire = p.native_wire(snapshot.signed_in && snapshot.profile_fresh && snapshot.storage_available);
            wire["clientAccess"] = serde_json::to_value(&snapshot.access).expect("access presentation");
            wire
        });
        let billing = self.inner.billing.snapshot(state.epoch);
        let checkout = self.inner.checkout.snapshot(state.epoch);
        let catalog = self.inner.catalog.snapshot(state.epoch);
        let window = self.inner.subscription.snapshot(state.epoch, &checkout, &billing, &catalog);
        let overview = crate::subscription::overview::SubscriptionOverview::from_snapshots(
            state.epoch, snapshot.signed_in, &billing, &checkout, &catalog, window["busy"] == true);
        let actions = crate::billing_actions::BillingActions::project(&snapshot, &billing, &checkout);
        serde_json::json!({"snapshot": snapshot, "tokens": tokens, "profile": profile,
            "pendingDeviceLimit": state.pending_device_limit, "devices": state.devices.list,
            "billingCatalog": catalog,
            "billingStatus": billing, "billingActions": actions, "checkout": checkout,
            "promotionWindow": self.promotion_snapshot(state.epoch),
            "subscriptionWindow": window, "subscriptionOverview": overview})
    }

    /// Worker/host shutdown stops in-flight HTTP, preserving durable credentials
    /// for the next launch. This is not a user logout and makes no storage writes.
    pub fn disconnect(&self) {
        let mut state = self.mutate();
        state.epoch += 1;
        state.devices = Default::default();
        self.inner.billing.identity_changed(state.epoch);
        state.tokens = None;
        state.profile = None;
        state.profile_fresh = false;
        state.style_profile_verified = false;
        state.storage_available = false;
        self.inner.boundary.send_replace(state.epoch);
    }

    pub(crate) fn state(&self) -> MutexGuard<'_, State> {
        // Poisoning is an implementation defect, not an excuse to reuse authority.
        self.inner.state.lock().expect("account state lock")
    }

    pub(crate) fn mutate(&self) -> MutexGuard<'_, State> {
        let mut state = self.state();
        state.revision += 1;
        state
    }

    pub(crate) fn changed(&self) {
        self.inner.changed.send_modify(|revision| *revision += 1);
    }

    /// Fence private host workflows against a completed account boundary.
    pub fn current(&self, epoch: u64) -> Result<()> {
        if self.state().epoch == epoch { Ok(()) } else { Err(Error::SessionChanged) }
    }

    pub(crate) fn lease(&self) -> Result<Lease> {
        let state = self.state();
        if !state.storage_available { return Err(Error::StorageUnavailable); }
        let tokens = state.tokens.clone().ok_or(Error::SignedOut)?;
        Ok(Lease { epoch: state.epoch, token_revision: state.token_revision, tokens,
            boundary: self.inner.boundary.subscribe() })
    }

    pub(crate) async fn send_scoped(&self, epoch: u64, mut boundary: watch::Receiver<u64>,
        request: crate::AccountRequest, bearer: Option<String>) -> Result<HttpResponse> {
        self.current(epoch)?;
        let response = tokio::select! {
            biased;
            _ = boundary.changed() => return Err(Error::SessionChanged),
            response = self.inner.transport.send(request, bearer) => response?,
        };
        self.current(epoch)?;
        Ok(response)
    }
}
