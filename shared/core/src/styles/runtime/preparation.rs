//! Foreground issues inside its own deadline; it never joins background work.
use super::Stylization;
use crate::{Error, Result};
impl Stylization {
    pub(super) async fn join_preparation(&self, epoch: u64, required: usize,
        _selected: bool, _online: bool) -> Result<u64> {
        self.current(epoch)?;
        if !(1..=crate::style_inventory::BATCH).contains(&required) { return Err(Error::InvalidRequest); }
        Ok(0)
    }
}
