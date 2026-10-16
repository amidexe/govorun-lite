//! Requests are retried within their own lifetime; abandoned passes are spent.
use super::foreground::Foreground;
use crate::Result;
#[derive(Debug)]
pub struct RecoveryStep { pub wire_attempted: bool, pub requires_recovery: bool, pub retry_after_millis: Option<u64> }
impl Foreground {
    pub async fn has_blocking_credit(&self, epoch: u64) -> Result<bool> {
        self.account.current(epoch)?; Ok(false)
    }
    pub async fn recover(&self, epoch: u64) -> Result<RecoveryStep> {
        self.account.current(epoch)?;
        Ok(RecoveryStep { wire_attempted: false, requires_recovery: false, retry_after_millis: None })
    }
}
