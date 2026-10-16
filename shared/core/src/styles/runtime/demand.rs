//! Foreground readiness is independent of background recovery and maintenance.
//! Only an empty/insufficient reserve joins the idempotent issuance journal.
use super::{commands::random_id, Stylization};
use crate::{style_pass_vault::presentation::Admission, Error, Result};

impl Stylization {
    pub(super) async fn demand_ready(&self, epoch: u64, required: usize) -> Result<()> {
        self.current(epoch)?;
        self.foreground.allowed(epoch)?;
        let view = self.observation(true, false).await?;
        if matches!(view.admission(required)?, Admission::Go) {
            return Ok(());
        }
        self.foreground.record_on_demand_issue();
        loop {
            self.current(epoch)?;
            let profile = self.account.profile().ok_or(Error::SignedOut)?;
            let access_end = profile.wire()["styleAccess"]["endsAt"]
                .as_str()
                .map(|value| {
                    chrono::DateTime::parse_from_rfc3339(value)
                        .ok()
                        .and_then(|date| u64::try_from(date.timestamp_millis()).ok())
                        .ok_or(Error::InvalidResponse)
                })
                .transpose()?;
            // Each journal is exact/idempotent; the entire chain stays inside
            // Foreground's original text-dependent deadline, never a new one.
            let outcome = self.issuance.warm(epoch, crate::style_inventory::BATCH, access_end, random_id()?).await;
            if let Ok(outcome) = &outcome {
                if let Some(limit) = outcome.quota { return Err(Error::StyleQuota(limit)); }
            }
            self.foreground.allowed(epoch)?;
            if matches!(self.observation(true, false).await?.admission(required)?, Admission::Go) {
                return Ok(());
            }
            let outcome = outcome?;
            if let Some(status) = outcome.rejected_status {
                return Err(Error::StyleIssuance { stage: "issue", status });
            }
            // A successful exact replay may already have supplied the other
            // foreground lease. A new batch serves this one; contention alone
            // is never reserve-empty, a retry budget or a restarted deadline.
            tokio::task::yield_now().await;
        }
    }
}
