use crate::{AccountRequest, AccountSession, Error, Future, HttpResponse, Method, Result};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, sync::Mutex};

#[path = "billing_management.rs"]
mod management;
pub use management::{BillingManagementCommand, BillingManagementSnapshot};
pub(crate) use management::visible as management_visible;
#[path = "billing_refund.rs"]
mod refund;
pub use refund::BillingRefundReason;
#[path = "billing_checkout.rs"]
mod checkout;
pub use checkout::BillingCheckoutPresentation;

pub trait BillingStatusSource: Send + Sync {
    fn current(&self, epoch: u64) -> Result<()>;
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>>;
    fn billing_changed(&self, _snapshot: &BillingStatusSnapshot) -> Result<()> { Ok(()) }
}

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BillingStatusSnapshot {
    pub epoch: u64,
    pub revision: u64,
    pub status: Option<Value>,
    pub fresh: bool,
    pub refreshing: bool,
    pub mutation_pending: bool,
    pub message: Option<String>,
    pub management: Option<BillingManagementSnapshot>,
    pub checkout: BillingCheckoutPresentation,
}
impl BillingStatusSnapshot {
    fn empty(epoch: u64, revision: u64) -> Self {
        Self { epoch, revision, status: None, fresh: false, refreshing: false, mutation_pending: false,
            message: None, management: None,
            checkout: BillingCheckoutPresentation::from_status(None, false) }
    }
}

struct State {
    snapshot: BillingStatusSnapshot,
    sequence: u64,
    latest_read: u64,
    mutation: u64,
    reads: HashSet<u64>,
    writes: HashSet<u64>,
    management_id: Option<u64>,
}
impl State {
    fn new(epoch: u64, revision: u64) -> Self {
        Self { snapshot: BillingStatusSnapshot::empty(epoch, revision), sequence: 0,
            latest_read: 0, mutation: 0, reads: HashSet::new(), writes: HashSet::new(), management_id: None }
    }
    fn changed(&mut self) -> BillingStatusSnapshot {
        self.snapshot.revision += 1;
        self.snapshot.refreshing = !self.reads.is_empty();
        self.snapshot.mutation_pending = !self.writes.is_empty();
        self.snapshot.checkout = BillingCheckoutPresentation::from_status(
            self.snapshot.status.as_ref(), self.snapshot.fresh);
        self.snapshot.clone()
    }
}

/// Owns confirmed subscription data, freshness and ordering across reads and
/// writes. A failed read retains information without presenting it as fresh.
/// It does not grant processing access, acknowledge checkout or retry HTTP.
pub struct BillingStatusService { state: Mutex<State> }
impl Default for BillingStatusService {
    fn default() -> Self { Self { state: Mutex::new(State::new(0, 0)) } }
}

impl BillingStatusService {
    pub fn tracks(request: &AccountRequest) -> bool {
        (request.method() == Method::Get && request.path() == "/v1/billing/status") ||
            (request.path().starts_with("/v1/billing/") &&
                ((request.method() == Method::Post && !request.path().ends_with("/preview")) ||
                 request.path().starts_with("/v1/billing/tbank/checkout/") ||
                 request.path().starts_with("/v1/billing/tbank/checkout-attempts/")))
    }

    pub fn snapshot(&self, epoch: u64) -> BillingStatusSnapshot {
        let state = self.state.lock().expect("billing status lock");
        if state.snapshot.epoch == epoch { state.snapshot.clone() }
        else { BillingStatusSnapshot::empty(epoch, state.snapshot.revision) }
    }

    pub(crate) fn identity_changed(&self, epoch: u64) {
        let mut state = self.state.lock().expect("billing status lock");
        if state.snapshot.epoch <= epoch {
            *state = State::new(epoch, state.snapshot.revision + 1);
        }
    }

    pub async fn request(&self, source: &dyn BillingStatusSource, epoch: u64,
        request: AccountRequest) -> Result<HttpResponse> {
        if !Self::tracks(&request) { return Err(Error::InvalidRequest); }
        source.current(epoch)?;
        let read = request.method() == Method::Get && request.path() == "/v1/billing/status";
        let discovery = request.method() == Method::Get && request.path() == "/v1/billing/tbank/checkout/latest";
        let refund = request.method() == Method::Post && request.path() == "/v1/billing/refund-requests";
        let cancel_plan = request.method() == Method::Post && request.path() == "/v1/billing/change" &&
            self.snapshot(epoch).management.is_some_and(|m|
                m.running && m.command == BillingManagementCommand::CancelScheduledPlanChange);
        let expects_status = read || matches!(request.path(),
            "/v1/billing/cancel-renewal" | "/v1/billing/payment-method/remove");
        let (id, mutation, snapshot, before_discovery) = {
            let mut state = self.state.lock().expect("billing status lock");
            if state.snapshot.epoch > epoch { return Err(Error::SessionChanged); }
            if state.snapshot.epoch != epoch { *state = State::new(epoch, state.snapshot.revision); }
            state.sequence += 1;
            let id = state.sequence;
            let before_discovery = discovery.then(|| state.snapshot.clone());
            if read {
                state.latest_read = id;
                state.reads.insert(id);
            } else {
                state.mutation += 1;
                state.writes.insert(id);
                // Discovery is a read. Keep confirmed presentation while it is
                // pending; the ordering fence still excludes concurrent writes.
                if !discovery {
                    state.snapshot.fresh = false;
                    state.snapshot.message = None;
                    if state.management_id.is_none() { state.snapshot.management = None; }
                }
            }
            (id, state.mutation, state.changed(), before_discovery)
        };
        let mut pending = Pending { service: self, source, epoch, id, mutation, read, before_discovery, finished: false };
        source.billing_changed(&snapshot)?;
        let result = source.request(epoch, request).await.and_then(|response| {
            if cancel_plan && (200..300).contains(&response.status) && response.body["mode"] != "none" {
                return Err(Error::InvalidResponse);
            }
            if (200..300).contains(&response.status) && (expects_status || response.body["subscription"].is_object()) {
                validate_billing_status(&response.body)?;
            }
            if refund && (200..300).contains(&response.status) {
                refund::validate(&response.body["refundRequest"])?;
            }
            Ok(response)
        });
        source.current(epoch)?;
        let confirmed = result.as_ref().ok().filter(|r| (200..300).contains(&r.status));
        let mut value = confirmed
            .filter(|r| r.body["subscription"].is_object()).map(|r| r.body.clone());
        let complete_status = value.is_some();
        if refund && confirmed.is_some() && value.is_none() {
            value = self.snapshot(epoch).status.map(|mut prior| {
                let refund = &confirmed.unwrap().body["refundRequest"];
                prior["subscription"]["refundRequest"] = refund.clone();
                if refund["renewalStopped"] == true {
                    prior["subscription"]["renewsAutomatically"] = Value::Bool(false);
                    prior["subscription"]["nextCharge"] = Value::Null;
                }
                prior
            });
        }
        let message = (refund && confirmed.is_some()).then(|| refund::ACKNOWLEDGED.to_owned());
        let empty_discovery = discovery && matches!(&result,
            Ok(HttpResponse { status: 404, .. }) | Err(Error::Http(404)));
        let (snapshot, accepted) = self.finish(&pending, value, complete_status, message, empty_discovery)
            .ok_or(Error::SessionChanged)?;
        pending.finished = true;
        source.billing_changed(&snapshot)?;
        // A superseded read must not return an obsolete response to a native DTO
        // consumer, even while its presentation is still being migrated.
        if read && !accepted { return Err(Error::SessionChanged); }
        if (refund || cancel_plan) && confirmed.is_some() && accepted {
            // These acknowledgements are not full status. Refund acceptance is
            // retained independently; plan cancellation needs the resulting status.
            let _ = Box::pin(self.request(source, epoch,
                AccountRequest::new(Method::Get, "/v1/billing/status", None)?)).await;
            source.current(epoch)?;
        }
        result
    }

    fn finish(&self, pending: &Pending<'_>, value: Option<Value>, complete_status: bool,
        message: Option<String>, empty_discovery: bool) -> Option<(BillingStatusSnapshot, bool)> {
        let mut state = self.state.lock().expect("billing status lock");
        if state.snapshot.epoch != pending.epoch { return None; }
        let accepted = state.mutation == pending.mutation &&
            if pending.read { state.latest_read == pending.id && state.writes.is_empty() }
            else { state.writes.len() == 1 };
        if pending.read { state.reads.remove(&pending.id); }
        else {
            state.writes.remove(&pending.id);
            state.mutation += 1;
        }
        if accepted {
            state.snapshot.fresh = complete_status;
            if let Some(value) = value { state.snapshot.status = Some(value); }
            if message.is_some() {
                state.snapshot.message = message;
                state.snapshot.acknowledge_refund();
            }
            if pending.read && complete_status { state.snapshot.reconcile_management(); }
            // An empty account-wide discovery has not changed the subscription.
            // Restore only under the same ordering fence; a concurrent mutation,
            // failed lookup or dropped future must never regain stale freshness.
            if let Some(prior) = pending.before_discovery.as_ref().filter(|_| empty_discovery) {
                state.snapshot.status = prior.status.clone();
                state.snapshot.fresh = prior.fresh;
                state.snapshot.message = prior.message.clone();
                state.snapshot.management = prior.management.clone();
            }
        }
        Some((state.changed(), accepted))
    }
}

struct Pending<'a> {
    service: &'a BillingStatusService,
    source: &'a dyn BillingStatusSource,
    epoch: u64,
    id: u64,
    mutation: u64,
    read: bool,
    before_discovery: Option<BillingStatusSnapshot>,
    finished: bool,
}
impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if !self.finished {
            if let Some((snapshot, _)) = self.service.finish(self, None, false, None, false) {
                if self.source.current(self.epoch).is_ok() {
                    let _ = self.source.billing_changed(&snapshot);
                }
            }
        }
    }
}

pub fn validate_billing_status(value: &Value) -> Result<()> {
    let subscription = &value["subscription"];
    if !subscription.is_object() || !subscription["state"].as_str().is_some_and(|state|
        !state.trim().is_empty() && state.len() <= 64 && !state.chars().any(char::is_control)) ||
        !subscription["renewsAutomatically"].is_boolean() ||
        value.get("starterGrant").is_some_and(|v| !v.is_null() && !v.is_object()) {
        return Err(Error::InvalidResponse);
    }
    if let Some(value) = subscription.get("refundRequest").filter(|v| !v.is_null()) { refund::validate(value)?; }
    Ok(())
}

impl BillingStatusSource for AccountSession {
    fn current(&self, epoch: u64) -> Result<()> {
        self.current(epoch)?;
        self.lease().map(|_| ())
    }
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        Box::pin(self.authorized_request_at(epoch, request))
    }
    fn billing_changed(&self, snapshot: &BillingStatusSnapshot) -> Result<()> {
        self.current(snapshot.epoch)?;
        drop(self.mutate());
        self.changed();
        Ok(())
    }
}
