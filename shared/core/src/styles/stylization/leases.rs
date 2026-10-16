//! Selected passes leave the disk pool before sending; cancellation burns them.
use crate::{style_pass_vault::{Action, Repository}, AccountSession, Result};
use std::sync::Arc;
pub(super) struct PendingGuard {
    vault: Arc<Repository>, account: AccountSession, epoch: u64, id: String, now: u64, settled: bool,
}
impl PendingGuard {
    pub(super) fn new(vault: Arc<Repository>, account: AccountSession, epoch: u64, id: String, now: u64) -> Self {
        Self { vault, account, epoch, id, now, settled: false }
    }
    pub(super) fn finished(&mut self) -> Result<()> { self.settled = true; Ok(()) }
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        if self.settled { return; }
        let (vault, account, epoch, id, now) = (self.vault.clone(), self.account.clone(), self.epoch, self.id.clone(), self.now);
        tokio::spawn(async move {
            let _ = vault.change(Action::Settle { id, restore: false, consumed: false }, now, false, false, &|| account.current(epoch).is_ok()).await;
        });
    }
}
