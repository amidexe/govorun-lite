//! One read-only view and refill plan from authoritative account facts and the
//! protected local inventory. No native DTO can override quotas or consent.
use super::{Additions, Envelope};
use crate::{style_inventory::Event, AccountSession, Error, Result};
use serde_json::Value;

pub struct Observation {
    facts: crate::style_window::StyleWindowFacts,
    pub window: Value,
    pub refill: Value,
    pub ready: usize,
    pub pending: usize,
    pub issuance_pending: bool,
}
pub(crate) enum Admission { Go, IssueNow }
pub fn observe(
    account: &AccountSession,
    vault: Option<&Envelope>,
    now: u64,
    explicit_demand: bool,
    background_allowed: bool,
    recovery_in_flight: bool,
) -> Result<Observation> {
    let observation = account.style_observation(now);
    let snapshot = observation.snapshot;
    let profile = observation.profile;
    let normalized = vault
        .map(|v| v.inventory(Additions::default(), Event::Normalize, now, true))
        .transpose()?;
    let known = normalized.is_some();
    let bound = normalized
        .as_ref()
        .is_some_and(|v| v.candidate.scope_epoch().ok().flatten().is_some());
    let ready = normalized.as_ref().map_or(0, |v| v.ready);
    let pending = normalized.as_ref().map_or(0, |v| v.pending);
    let issuance_pending = normalized.as_ref().is_some_and(|v| v.issuance_pending);
    let source = normalized.as_ref().map(|v| v.candidate.stored_value());
    let count = |name: &str| {
        source
            .and_then(|v| v[name].as_array())
            .map_or(0, |v| v.len()) as u64
    };
    let issuance = source
        .and_then(|v| v["issuance"]["requests"].as_array())
        .map_or(0, |v| v.len()) as u64;
    let ledger = normalized
        .as_ref()
        .map(|v| v.candidate.usage())
        .transpose()?;
    let profile = profile.as_ref().map_or(&Value::Null, |p| p.wire());
    let reset = profile["styleAccess"]["dailyResetsAt"]
        .as_str()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .is_some_and(|v| v.timestamp_millis() <= now.min(i64::MAX as u64) as i64);
    let mut facts = crate::style_window::StyleWindowFacts {
        presentation_access: None,
        signed_in: snapshot.signed_in,
        profile_fresh: snapshot.profile_fresh,
        entitlement_active: snapshot.access.entitlement_active && observation.access_current,
        processing_ready: snapshot.access.processing_ready && observation.access_current,
        free_access_available: snapshot.access.free_access_available,
        access_type: String::new(),
        access_active: false,
        balance_kind: String::new(),
        starter_allocated: false,
        daily_limit: 0,
        total_limit: 0,
        remaining_today: 0,
        remaining_total: 0,
        usage_accounting: false,
        usage_fresh: observation.usage_fresh,
        pending_usage_today: 0,
        pending_usage_total: 0,
        usage_ledger: ledger,
        usage_scope: None,
        local_known: known,
        local_bound: bound,
        ready_credits: ready as u64,
        pending_credits: pending as u64,
        issuance_in_flight: issuance_pending,
        recovery_in_flight,
        daily_reset_passed: reset,
        reserve_return: profile["styleAccess"]["reserveReturnProtocol"] == "v1_receipt_deposit",
    };
    crate::style_window::bind_profile(&mut facts, profile)?;
    let mut context = crate::style_refill::Context {
        reserve_credits: crate::style_refill::TARGET,
        issuance_remaining_today: 0,
        issuance_remaining_total: 0,
        replacement_receipts: count("replacementReceipts"),
        pending_issuance_credits: issuance,
        retiring_credits: count("retiring"),
        explicit_demand,
        background_allowed,
        issuance_counters_observed: false,
    };
    crate::style_refill::bind_profile(&mut context, profile)?;
    let access_ceiling=profile["styleAccess"]["endsAt"].as_str()
        .and_then(|text|chrono::DateTime::parse_from_rfc3339(text).ok())
        .and_then(|time|u64::try_from(time.timestamp_millis()).ok()).map(crate::style_issuance::cohort_end);
    // A pass already at the access ceiling cannot be replaced by a later
    // cohort. Count it until TAKE_MARGIN instead of repeatedly minting the
    // same short-lived cohort. This never changes admission or signed expiry.
    let protected_ready=normalized.as_ref().map(|value|value.candidate.ready_expiries())
        .transpose()?.unwrap_or_default().into_iter()
        .filter(|expiry|*expiry>now.saturating_add(crate::style_refill::PREFILL_LEAD_MILLIS)
            || access_ceiling==Some(*expiry)).count() as u64;
    let refill = serde_json::to_value(crate::style_refill::plan_for_reserve(facts.clone(), context,protected_ready)?)
        .map_err(|_| Error::InvalidResponse)?;
    let window = serde_json::to_value(crate::style_window::project(facts.clone())?)
        .map_err(|_| Error::InvalidResponse)?;
    Ok(Observation {
        facts,
        window,
        refill,
        ready,
        pending,
        issuance_pending,
    })
}

impl Observation {
    /// Pure working-plane decision. Refresh/probe states cannot deny a request;
    /// only confirmed access and balance can. Custody excludes fenced passes.
    pub(crate) fn admission(&self, required: usize) -> Result<Admission> {
        if !self.facts.signed_in { return Err(Error::SignedOut); }
        if !self.facts.entitlement_active || !self.facts.processing_ready {
            return Err(Error::StyleAccess(crate::stylization::AccessFailure::NoAccess));
        }
        use crate::stylization::AccessFailure;
        match self.window["availability"].as_str() {
            Some("dailyLimit") => return Err(Error::StyleAccess(AccessFailure::DailyLimit)),
            Some("periodLimit") => return Err(Error::StyleAccess(AccessFailure::PeriodLimit)),
            _ => {}
        }
        Ok(if self.ready >= required { Admission::Go } else { Admission::IssueNow })
    }
    /// Read-only UI fact: no unavailable vault is presented as a working one.
    pub fn storage_available(&self) -> bool { !self.facts.signed_in || self.facts.local_known }
    /// Uses the same bound inventory and fresh account facts as the screen.
    /// Callers supply demand phase and physical connectivity, never a quota.
    pub fn preparation(
        &self,
        phase: &str,
        selected: bool,
        online: bool,
        required: usize,
    ) -> Result<Value> {
        let context: crate::style_preparation::Context = serde_json::from_value(serde_json::json!({
            "phase":phase,"enabled":selected,"customServer":false,"networkAvailable":online,
            "profileFresh":self.facts.profile_fresh,"refreshRequested":false,"requiredCredits":required
        })).map_err(|_| Error::InvalidRequest)?;
        serde_json::to_value(crate::style_preparation::plan(self.facts.clone(), context)?)
            .map_err(|_| Error::InvalidResponse)
    }
}
