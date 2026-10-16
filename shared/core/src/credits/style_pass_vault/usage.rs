//! Canonical numeric reservations use the same policy in both current formats.
//! FlatV4 physically records pending usage in lease fields instead of usage.pending.
use super::{format::Format, Envelope};
use crate::{usage_ledger as usage, Error, Result};
use serde_json::Value;
impl Envelope {
    pub fn usage(&self) -> Result<usage::Ledger> {
        let mut ledger: usage::Ledger = serde_json::from_value(self.source["usage"].clone())
            .map_err(|_| Error::StorageInvalid)?;
        if self.format != Format::LedgerV5 {
            if !ledger.pending.is_empty() {
                return Err(Error::StorageInvalid);
            }
            for lease in self.source["pending"]
                .as_array()
                .ok_or(Error::StorageInvalid)?
            {
                let scopes = lease["passes"]
                    .as_array()
                    .ok_or(Error::StorageInvalid)?
                    .iter()
                    .map(|p| p["usageScopeId"].as_str().map(str::to_owned))
                    .collect();
                ledger = usage::apply(
                    ledger,
                    usage::Event::Begin {
                        lease_id: lease["id"].as_str().ok_or(Error::StorageInvalid)?.into(),
                        day: lease["usageDay"].as_str().map(str::to_owned),
                        scopes,
                        admitted_scope: None,
                    },
                )?
                .ledger;
            }
        }
        usage::apply(ledger, usage::Event::Validate)
            .map(|v| v.ledger)
            .map_err(|_| Error::StorageInvalid)
    }
    pub(super) fn store_usage(&mut self, mut ledger: usage::Ledger) -> Result<()> {
        if self.format != Format::LedgerV5 {
            ledger.pending.clear();
        }
        let previous: usage::Ledger = serde_json::from_value(self.source["usage"].clone())
            .map_err(|_| Error::StorageInvalid)?;
        if previous != ledger {
            self.source["usage"] =
                serde_json::to_value(ledger).map_err(|_| Error::StorageInvalid)?;
        }
        Ok(())
    }
    /// Protected scheduling metadata, never a renderer or diagnostic payload.
    pub(crate) fn ready_expiries(&self) -> Result<Vec<u64>> {
        let ledger=if self.format==Format::LedgerV5 {&self.source["ledger"]} else {&self.source};
        ledger["available"].as_array().ok_or(Error::StorageInvalid)?.iter()
            .map(|pass|self.format.date_millis(&pass["expiresAt"])).collect()
    }
    pub(crate) fn stored_leases(&self) -> Result<&[Value]> {
        let ledger = if self.format == Format::LedgerV5 {
            &self.source["ledger"]
        } else {
            &self.source
        };
        ledger["pending"]
            .as_array()
            .map(Vec::as_slice)
            .ok_or(Error::StorageInvalid)
    }
}
