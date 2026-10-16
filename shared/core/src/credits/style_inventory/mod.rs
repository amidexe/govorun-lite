mod model;
use crate::{Error, Result};
pub use model::{Decision, Event, Journal, Lease, Pass, Receipt, State};
use std::collections::HashSet;

pub(crate) const BATCH: usize = crate::credit_limits::BATCH_CREDITS;
const MAX_PASSES: usize = 4_096;
const MAX_PENDING: usize = 512;
pub(crate) const RETIREMENT_WINDOW: u64 = 30 * 60 * 1_000;
const RETIREMENT_GRACE: u64 = 35 * 24 * 60 * 60 * 1_000;
const RECEIPT_LIFETIME: u64 = 47 * 60 * 60 * 1_000;
pub(crate) const TAKE_MARGIN: u64 = 60 * 1_000;
const MAX_DATE: u64 = 253_402_300_799_999;

/// Compute a candidate only. The native port must fence the input snapshot,
/// durably store this result, and only then publish it or admit a request.
pub fn transition(
    mut state: State,
    event: Event,
    now: u64,
    retirement_available: bool,
) -> Result<Decision> {
    validate(&state, now)?;
    normalize(&mut state, now, retirement_available);
    let mut applied = true;
    let mut merged = false;
    let mut selected = Vec::new();
    match event {
        Event::Normalize => {}
        Event::ChangeScope { compatible } => {
            applied = !compatible || state.scope_epoch.is_none();
            if applied {
                let epoch = state.scope_epoch.unwrap_or(0);
                if epoch >= i64::MAX as u64 {
                    return Err(Error::InvalidRequest);
                }
                state.scope_epoch = Some(epoch + 1);
                // A revoked scope fences usable passes, it does not erase
                // paid stored pass state or exact return/deposit evidence. None of these
                // old passes can return to available in the new scope.
                state.retiring.append(&mut state.available);
                // Pending leases and the exact issue journal keep their old
                // epoch for usage confirmation; neither can replenish the new reserve.
            }
        }
        Event::Lease { id, count } => {
            if !(1..=BATCH).contains(&count)
                || id.is_empty()
                || state.pending.iter().any(|lease| lease.id == id)
            {
                return Err(Error::InvalidRequest);
            }
            if state.scope_epoch.is_none()
                || state.pending.len() >= MAX_PENDING
                || state.available.len() < count
            {
                applied = false;
            } else {
                state.available.sort_by_key(|pass| pass.expires_at_millis);
                let passes: Vec<_> = state.available.drain(..count).collect();
                selected = passes.iter().map(|p| p.id.clone()).collect();
                state.pending.push(model::Lease {
                    id,
                    passes,
                    scope_epoch: state.scope_epoch,
                });
            }
        }
        Event::Settle { id, restore } => {
            if let Some(index) = state.pending.iter().position(|lease| lease.id == id) {
                let lease = state.pending.remove(index);
                merged = restore
                    && state.scope_epoch.is_some()
                    && lease.scope_epoch == state.scope_epoch;
                if merged {
                    state.available.extend(lease.passes);
                } else if restore {
                    // Released old-scope capabilities still have paid value,
                    // but cannot authorize requests in the current scope.
                    state.retiring.extend(lease.passes);
                }
            } else {
                applied = false;
            }
        }
        Event::BeginIssuance { journal } => {
            if !(1..=BATCH).contains(&journal.count)
                || journal.id.is_empty()
                || journal.receipts.len() > journal.count
                || !unique(&journal.receipts)
            {
                return Err(Error::InvalidRequest);
            }
            applied = state.issuance.is_none()
                && state.scope_epoch.is_some()
                && journal.scope_epoch == state.scope_epoch
                && journal.receipts.iter().all(|id| !state.deposit_receipts.contains(id))
                && journal
                    .receipts
                    .iter()
                    .all(|id| state.receipts.iter().any(|r| &r.id == id));
            if applied {
                state.issuance = Some(journal);
            }
        }
        Event::FinishIssuance { id, passes } => {
            applied = state.issuance.as_ref().is_some_and(|j| j.id == id);
            if applied {
                let journal = state.issuance.take().ok_or(Error::InvalidRequest)?;
                if passes.len() != journal.count
                    || passes.iter().any(|p| !valid_pass(p))
                    || !unique(&passes.iter().map(|p| p.id.clone()).collect::<Vec<_>>())
                {
                    return Err(Error::InvalidResponse);
                }
                merged = state.scope_epoch.is_some() && journal.scope_epoch == state.scope_epoch;
                if merged {
                    state.available.extend(passes);
                } else {
                    state.retiring.extend(passes);
                }
                state.receipts.retain(|r| !journal.receipts.contains(&r.id));
            }
        }
        Event::DiscardIssuance {
            id,
            scope_epoch,
            discard_receipts,
        } => {
            applied = state
                .issuance
                .as_ref()
                .is_some_and(|j| j.id == id && j.scope_epoch == scope_epoch);
            if applied {
                let journal = state.issuance.take().ok_or(Error::InvalidRequest)?;
                if discard_receipts {
                    state.receipts.retain(|r| !journal.receipts.contains(&r.id));
                }
            }
        }
        Event::SelectReceipts { count } => {
            if count > BATCH {
                return Err(Error::InvalidRequest);
            }
            selected = state
                .receipts
                .iter()
                .filter(|r| !state.deposit_receipts.contains(&r.id))
                .take(count)
                .map(|r| r.id.clone())
                .collect();
        }
        Event::Retire {
            scope_epoch,
            selected: tokens,
            receipts,
            discard,
        } => {
            if tokens.is_empty()
                || tokens.len() > BATCH
                || !unique(&tokens)
                || (!discard
                    && (tokens.len() != receipts.len()
                        || !unique(&receipts.iter().map(|r| r.id.clone()).collect::<Vec<_>>())
                        || receipts
                            .iter()
                            .any(|r| r.id.is_empty() || r.received_at_millis > MAX_DATE)))
            {
                return Err(Error::InvalidResponse);
            }
            applied = state.scope_epoch == Some(scope_epoch)
                && (discard
                    || tokens
                        .iter()
                        .all(|id| state.retiring.iter().any(|p| &p.id == id)));
            if applied {
                state.retiring.retain(|p| !tokens.contains(&p.id));
                if !discard {
                    let mut known: HashSet<_> =
                        state.receipts.iter().map(|r| r.id.clone()).collect();
                    state
                        .receipts
                        .extend(receipts.into_iter().filter(|r| known.insert(r.id.clone())));
                }
            }
        }
        Event::ReturnUnused => state.retiring.append(&mut state.available),
        Event::SelectDeposit => {
            selected = state.deposit_receipts.clone();
            if selected.is_empty() && state.issuance.is_none() {
                if let Some(group) = state.receipts.iter().find_map(|r| r.group.as_ref()) {
                    selected = state
                        .receipts
                        .iter()
                        .filter(|r| r.group.as_ref() == Some(group))
                        .take(BATCH)
                        .map(|r| r.id.clone())
                        .collect();
                }
            }
        }
        Event::CompleteDeposit {
            scope_epoch,
            receipts,
        } => {
            applied = state.scope_epoch.is_some_and(|current|scope_epoch<=current)
                && receipts == state.deposit_receipts;
            if applied {
                state.receipts.retain(|r| !receipts.contains(&r.id));
                state.deposit_receipts.clear();
            }
        }
    }
    normalize(&mut state, now, retirement_available);
    validate(&state, now)?;
    let bound = state.scope_epoch.is_some();
    let ready = if bound { state.available.len() } else { 0 };
    let pending = if bound {
        state
            .pending
            .iter()
            .filter(|p| p.scope_epoch == state.scope_epoch)
            .map(|p| p.passes.len())
            .sum::<usize>()
            + state.retiring.len()
            + state.receipts.len()
    } else {
        0
    };
    let issuance_pending = bound
        && state
            .issuance
            .as_ref()
            .is_some_and(|j| j.scope_epoch == state.scope_epoch);
    let reserve_ready = ready >= BATCH && state.receipts.is_empty();
    let retirement = if bound {
        state
            .retiring
            .iter()
            .take(BATCH)
            .map(|p| p.id.clone())
            .collect()
    } else {
        vec![]
    };
    Ok(Decision {
        state,
        selected,
        retirement,
        applied,
        merged,
        ready,
        pending,
        issuance_pending,
        reserve_ready,
    })
}

fn normalize(state: &mut State, now: u64, retirement_available: bool) {
    let cutoff = now.saturating_add(TAKE_MARGIN);
    let withdrawal = now.saturating_add(RETIREMENT_WINDOW);
    let pending: HashSet<_> = state
        .pending
        .iter()
        .flat_map(|l| &l.passes)
        .map(|p| p.id.clone())
        .collect();
    if retirement_available {
        // A full newer request is a replacement, not an expected network result.
        // Without it the nearest cohort remains usable until TAKE_MARGIN.
        let replacements = state.available.iter().filter(|p|
            p.expires_at_millis > withdrawal && !pending.contains(&p.id)).count();
        let mut budget = if replacements >= BATCH { replacements } else { 0 };
        state.available.sort_by_key(|p| p.expires_at_millis);
        state.retiring.extend(
            state
                .available
                .iter()
                .filter(|p| {
                    if pending.contains(&p.id)
                        || p.expires_at_millis > withdrawal
                        || p.expires_at_millis.saturating_add(RETIREMENT_GRACE) <= now {
                        return false;
                    }
                    if p.expires_at_millis <= cutoff { return true; }
                    if budget == 0 { return false; }
                    budget -= 1;
                    true
                })
                .cloned(),
        );
    }
    let mut retiring = HashSet::new();
    state.retiring.retain(|p| {
        !pending.contains(&p.id)
            && retiring.insert(p.id.clone())
            // Scope-limited passes lose admission, not their anonymous return
            // value. The server's retirement grace applies to every signed pass.
            && p.expires_at_millis.saturating_add(RETIREMENT_GRACE) > now
    });
    state.retiring.sort_by_key(|p| p.expires_at_millis);
    let retiring: HashSet<_> = state.retiring.iter().map(|p| &p.id).collect();
    let mut available = HashSet::new();
    state.available.retain(|p| {
        p.expires_at_millis > cutoff
            && !pending.contains(&p.id)
            && !retiring.contains(&p.id)
            && available.insert(p.id.clone())
    });
    state
        .available
        .sort_by_key(|p| std::cmp::Reverse(p.expires_at_millis));
    let mut protected: HashSet<_> = state.deposit_receipts.iter().collect();
    if let Some(journal) = &state.issuance {
        protected.extend(&journal.receipts);
    }
    let mut receipts = HashSet::new();
    state.receipts.retain(|r| {
        (r.received_at_millis.saturating_add(RECEIPT_LIFETIME) > now || protected.contains(&r.id))
            && receipts.insert(r.id.clone())
    });
}

fn unique(ids: &[String]) -> bool {
    ids.iter().collect::<HashSet<_>>().len() == ids.len()
}
fn valid_pass(p: &Pass) -> bool {
    !p.id.is_empty() && p.expires_at_millis > 0 && p.expires_at_millis <= MAX_DATE
}
fn validate(state: &State, now: u64) -> Result<()> {
    let passes: Vec<_> = state
        .available
        .iter()
        .chain(&state.retiring)
        .chain(state.pending.iter().flat_map(|p| &p.passes))
        .collect();
    if now > MAX_DATE
        || passes.len() > MAX_PASSES
        || state.pending.len() > MAX_PENDING
        || state.receipts.len() > MAX_PASSES
        || passes.iter().any(|p| !valid_pass(p))
        || state
            .pending
            .iter()
            .any(|p| p.id.is_empty() || !(1..=BATCH).contains(&p.passes.len()))
        || !unique(
            &state
                .pending
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>(),
        )
        || state
            .receipts
            .iter()
            .any(|r| r.id.is_empty() || r.received_at_millis > MAX_DATE)
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}
