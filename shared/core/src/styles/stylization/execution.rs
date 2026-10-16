//! Durable foreground admission and usage confirmation. Retry/status parsing stays in
//! the foreground owner; platform code performs no credit transitions.
use super::{
    foreground::{Foreground, Output, Stage},
    leases::PendingGuard,
    Request,
};
use crate::{style_inventory::Event, style_pass_vault::Action, Error, Result};
use serde_json::{json, Value};
use tokio::time::{Duration, Instant};
impl Foreground {
    pub(super) async fn run_job(
        &self,
        epoch: u64,
        mut payload: Value,
        lease_id: String,
        request_id: String,
        preparation_millis: u64,
        deadline_cap: u64,
        notes: &super::outage::AttemptNotes,
        progress: &(dyn Fn(Stage) + Send + Sync),
        readiness: crate::Future<'_, Result<()>>,
    ) -> Result<Output> {
        self.attempt_current(epoch)?;
        if !crate::request::uuid(&lease_id)
            || !crate::request::uuid(&request_id)
            || !payload.is_object()
            || payload.get("stylePasses").is_some()
            || payload.get("clientReliability").is_some()
        {
            return Err(Error::InvalidRequest);
        }
        let text = payload["text"].as_str().ok_or(Error::InvalidRequest)?;
        let admitted = crate::style_text::admit(text.encode_utf16().collect())?;
        let credits = super::foreground::number(&admitted, "requiredCredits")? as usize;
        let normal_budget = super::foreground::number(&admitted, "deadlineMillis")?;
        // Read once from the already cached profile: no freshness gate or I/O.
        // Missing/invalid optional metadata preserves the old length budget.
        let slow_percent = self.account.profile().and_then(|p|
            p.wire()["styleSlowBudgetPercent"].as_u64())
            .filter(|p| (1..=100).contains(p)).unwrap_or(100);
        let limit = normal_budget
            .min(deadline_cap)
            .checked_sub(preparation_millis)
            .filter(|v| *v > 0)
            .ok_or(Error::StyleTransport(super::Failure::Deadline))?;
        let started = Instant::now();
        let end = started + Duration::from_millis(limit);
        progress(Stage::Checking);
        let mut boundary = self.account.subscribe_boundary();
        // Bound live foreground operations, never ambiguous durable leases or
        // control-plane work. Waiting consumes this attempt's original budget.
        let _active = tokio::select! {
            biased;
            _ = boundary.changed() => return Err(Error::SessionChanged),
            slot = tokio::time::timeout_at(end, self.active.acquire()) => slot
                .map_err(|_| {
                    notes.1.timeout("overall", limit);
                    Error::StyleTransport(super::Failure::Deadline)
                })?
                .map_err(|_| Error::SessionChanged)?,
        };
        let ready = tokio::select! {
            biased;
            _ = boundary.changed() => return Err(Error::SessionChanged),
            ready = tokio::time::timeout_at(end, readiness) => ready,
        };
        self.attempt_current(epoch)?;
        ready.map_err(|_| {
            notes.1.timeout("overall", limit);
            Error::StyleTransport(super::Failure::Deadline)
        })??;
        let admission_started = Instant::now();
        let acceptance_end = end.min(
            admission_started
                + Duration::from_millis(super::foreground::number(
                    &admitted,
                    "admissionBudgetMillis",
                )?),
        );
        let acceptance_limit = acceptance_end.duration_since(admission_started).as_millis() as u64;
        let acceptance_scope = if acceptance_end == end {
            "overall"
        } else {
            "admission"
        };
        // Serialize only the local checkout transaction. The inventory's
        // bounded pending count and per-lease book fence stored pass state, not other I/O.
        let admission = tokio::time::timeout_at(acceptance_end, self.admission.lock())
            .await
            .map_err(|_| {
                notes.1.timeout(acceptance_scope, acceptance_limit);
                Error::StyleTransport(super::Failure::Deadline)
            })?;
        let lease = {
            let normalized = self
                .mutate(epoch, Action::Inventory(Event::Normalize))
                .await?;
            let state = normalized.candidate;
            let observed = crate::style_pass_vault::presentation::observe(
                &self.account,
                Some(&state),
                (self.clock)(),
                true,
                false,
                false,
            )?;
            observed.admission(credits)?;
            let profile = self.account.profile().ok_or(Error::SignedOut)?;
            let usage = &profile.wire()["styleAccess"]["usageBalance"];
            let result = self
                .mutate(
                    epoch,
                    Action::Lease {
                        id: lease_id.clone(),
                        count: credits,
                        day: usage["day"].as_str().map(str::to_owned),
                        admitted_scope: usage["scopeId"].as_str().map(str::to_owned),
                    },
                )
                .await?;
            if !result.applied || result.selected.len() != credits {
                return Err(Error::StyleAccess(super::AccessFailure::ReserveEmpty));
            }
            result.selected
        };
        drop(admission);
        let mut guard = PendingGuard::new(self.vault.clone(), self.account.clone(), epoch, lease_id.clone(), (self.clock)());
        let outcome_pass = lease.first().cloned().ok_or(Error::StorageInvalid)?;
        payload["stylePasses"] = json!(lease);
        let mut detached = self.reporting().detach(epoch)?;
        if let Some(detached) = &detached {
            payload["clientReliability"] = detached.wire()?;
        }
        if super::foreground::remaining(end) == 0 {
            self.mutate(
                epoch,
                Action::Settle {
                    id: lease_id.clone(),
                    restore: true,
                    consumed: false,
                },
            )
            .await?;
            guard.finished()?;
            notes.1.timeout("overall", limit);
            return Err(Error::StyleTransport(super::Failure::Deadline));
        }
        // Serialization/admission failure before a start attempt can restore.
        if let Err(error) = Request::start(
            payload.clone(),
            request_id.clone(),
            Duration::from_millis(super::foreground::remaining(acceptance_end).max(1)),
        ) {
            self.mutate(
                epoch,
                Action::Settle {
                    id: lease_id.clone(),
                    restore: true,
                    consumed: false,
                },
            )
            .await?;
            guard.finished()?;
            return Err(error);
        }
        let job = tokio::time::timeout_at(end, async {
            let first = self
                .start(
                    epoch,
                    payload,
                    request_id,
                    acceptance_end,
                    (acceptance_scope, acceptance_limit),
                    &lease_id,
                    progress,
                    notes,
                    &mut detached,
                )
                .await?;
            if first["kind"] == "processing" {
                progress(Stage::Processing);
                self.poll(
                    epoch,
                    first["jobToken"]
                        .as_str()
                        .ok_or(Error::InvalidResponse)?
                        .into(),
                    end,
                    notes,
                )
                .await
            } else {
                Ok(first)
            }
        })
        .await
        .map_err(|_| {
            notes.1.timeout("overall", limit);
            Error::StyleTransport(super::Failure::Deadline)
        }).and_then(|result| result);
        let job = match job {
            Ok(job) => job,
            Err(error) => {
                let _ = self.mutate(epoch, Action::Settle { id: lease_id.clone(), restore: false, consumed: false }).await;
                guard.finished()?;
                return Err(error);
            }
        };
        self.attempt_current(epoch)?;
        notes.1.server(&job);
        let response_phase = notes.1.current_phase();
        notes.1.phase("finalize");
        let settlement = job["settlement"].as_str().ok_or(Error::InvalidResponse)?;
        match settlement {
            "consume" | "restore" | "discard" => {
                self.mutate(
                    epoch,
                    Action::Settle {
                        id: lease_id,
                        restore: settlement == "restore",
                        consumed: settlement == "consume",
                    },
                )
                .await?;
                guard.finished()?;
            }
            "defer" => {
                self.mutate(epoch, Action::Settle { id: lease_id, restore: false, consumed: false }).await?;
                guard.finished()?;
            }
            _ => {}
        }
        if job["kind"] == "rejected" {
            notes.1.phase(response_phase);
            return Err(Error::Http(
                job["httpStatus"].as_u64().ok_or(Error::InvalidResponse)? as u16,
            ));
        }
        if job["kind"] != "completed" {
            notes.1.phase(response_phase);
            return Err(super::foreground::service_error(&job));
        }
        let text = job["text"].as_str().ok_or(Error::InvalidResponse)?.into();
        let elapsed = started.elapsed().as_millis();
        Ok(Output {
            text,
            outcome_pass,
            latency_bucket: super::foreground::latency(elapsed),
            // Reuse the admitted length-dependent budget. A normal 20s result
            // for long text must not become a diagnostic incident at a fixed10s.
            slow: elapsed.saturating_add(u128::from(preparation_millis)).saturating_mul(100)
                >= u128::from(normal_budget) * u128::from(slow_percent),
        })
    }
}
