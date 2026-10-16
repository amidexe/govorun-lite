//! Ephemeral job identities are never reused when cancellation clears a queue.
//! Executor tasks and audio stay in the host.
use crate::{Error, Result};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JobId(u64);
#[derive(Default)]
pub struct Jobs {
    next: u64,
    pending: BTreeSet<JobId>,
}
impl Jobs {
    pub fn enqueue(&mut self) -> Result<JobId> {
        self.next = self.next.checked_add(1).ok_or(Error::InvalidRequest)?;
        let id = JobId(self.next);
        self.pending.insert(id);
        Ok(id)
    }
    pub fn complete(&mut self, id: JobId) -> bool {
        self.pending.remove(&id)
    }
    pub fn cancel_all(&mut self) {
        self.pending.clear();
    }
    pub fn count(&self) -> usize {
        self.pending.len()
    }
}
