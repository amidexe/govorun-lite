//! Bounded, process-local counts. No timestamps, stable IDs or event history.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const PER_REASON: u32 = 20;
const TOTAL: u32 = 50;
const REASONS: [&str; 12] = [
    "no_change",
    "request_failed",
    "server_busy",
    "server_timeout",
    "server_unavailable",
    "invalid_response",
    "insertion_failed",
    "style_pass_reserve_empty",
    "style_pass_issue_failed",
    "style_pass_recovery_pending",
    "account_state_changed",
    "transport_failed",
];

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Batch {
    schema_version: u8,
    failures: BTreeMap<String, u32>,
}
impl Batch {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.failures.is_empty()
            || self.failures.len() > REASONS.len()
            || self.failures.iter().any(|(reason, count)| {
                !REASONS.contains(&reason.as_str()) || *count == 0 || *count > PER_REASON
            })
            || self.failures.values().copied().sum::<u32>() > TOTAL
        {
            return Err(Error::InvalidRequest);
        }
        Ok(())
    }
}
pub struct Lease {
    pub batch: Batch,
    generation: u64,
}
#[derive(Default)]
pub struct Counters {
    pending: BTreeMap<String, u32>,
    generation: u64,
}
impl Counters {
    pub(super) fn pending_report(&self) -> bool { !self.pending.is_empty() }
    pub fn record(&mut self, reason: &str) -> bool {
        if !super::failure::technical(reason) || self.pending.values().sum::<u32>() >= TOTAL {
            return false;
        }
        let count = self.pending.entry(reason.into()).or_default();
        if *count >= PER_REASON {
            return false;
        }
        *count += 1;
        true
    }
    pub fn detach(&mut self) -> Option<Lease> {
        if self.pending.is_empty() {
            return None;
        }
        Some(Lease {
            batch: Batch {
                schema_version: 1,
                failures: std::mem::take(&mut self.pending),
            },
            generation: self.generation,
        })
    }
    /// Only proven pre-wire failure. A possibly sent batch is never retried.
    pub fn restore_before_wire(&mut self, lease: Lease) {
        if lease.generation != self.generation || lease.batch.validate().is_err() {
            return;
        }
        self.merge(lease.batch.failures);
    }
    fn merge(&mut self, counts:BTreeMap<String,u32>) {
        let mut capacity = TOTAL - self.pending.values().sum::<u32>();
        for (reason,count) in counts {
            if capacity==0 {break;}
            if !REASONS.contains(&reason.as_str()) || count==0 {continue;}
            let pending=self.pending.entry(reason).or_default();
            let added=count.min(PER_REASON-*pending).min(capacity); *pending+=added;capacity-=added;
        }
    }
    pub(crate) fn binding(pending:BTreeMap<String,u32>,reason:Option<&str>,restore:BTreeMap<String,u32>) -> BTreeMap<String,u32> {
        let mut state=Self::default();state.merge(pending);
        if let Some(reason)=reason {state.record(reason);}state.merge(restore);state.pending
    }
    pub(crate) fn taxonomy() -> serde_json::Value {serde_json::json!({"reasons":REASONS,"perReason":PER_REASON,"total":TOTAL})}

    pub fn end_session(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending.clear();
    }
}
