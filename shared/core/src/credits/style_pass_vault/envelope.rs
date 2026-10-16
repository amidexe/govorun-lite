//! Strict read of both current wire layouts. The original JSON values (including
//! date representation and lease metadata) are retained for subsequent writes.
use super::format::{fields, text, Format};
use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Envelope {
    pub(super) source: Value,
    pub(super) format: Format,
}
impl Envelope {
    pub fn decode(raw: &str, format: Format) -> Result<Self> {
        if raw.len() > 4 * 1024 * 1024 {
            return Err(Error::StorageInvalid);
        }
        let source: Value = serde_json::from_str(raw).map_err(|_| Error::StorageInvalid)?;
        let this = Self { source, format };
        this.validate()?;
        Ok(this)
    }
    /// Still a protected value. Do not publish it as a UI or diagnostic snapshot.
    pub fn stored_value(&self) -> &Value {
        &self.source
    }
    pub fn format(&self) -> Format {
        self.format
    }
    pub fn encoded(&self) -> Result<String> {
        self.validate()?;
        let raw = self.source.to_string();
        if raw.len() > 4 * 1024 * 1024 {
            return Err(Error::StorageInvalid);
        }
        Ok(raw)
    }
    fn validate(&self) -> Result<()> {
        let root = &self.source;
        if self.format == Format::AndroidV8 {
            super::android_v8::validate(root.clone())?;
            return Ok(());
        }
        super::binding::validate_keys(root)?;
        let (available, pending, quarantined, epoch) = match self.format {
            Format::AndroidV8 => unreachable!("validated above"),
            Format::LedgerV5 => {
                fields(
                    root,
                    &[
                        "version",
                        "ledger",
                        "issuance",
                        "retiring",
                        "replacementReceipts",
                        "usage",
                        "scopeEpoch",
                        "quarantinedPendingLeaseIDs",
                        "issuanceScopeEpoch",
                    ],
                    &["reserveDeposit", "scopeKey", "identityKey"],
                )?;
                if root["version"] != 5 {
                    return Err(Error::StorageInvalid);
                }
                fields(&root["ledger"], &["available", "pending"], &[])?;
                let epoch = optional_epoch(&root["scopeEpoch"])?;
                let issuance_epoch = optional_epoch(&root["issuanceScopeEpoch"])?;
                if issuance_epoch.is_some_and(|e| epoch.is_none_or(|current| e > current))
                    || root["issuance"].is_null() && issuance_epoch.is_some()
                {
                    return Err(Error::StorageInvalid);
                }
                let quarantined = list(&root["quarantinedPendingLeaseIDs"], 512)?;
                (
                    &root["ledger"]["available"],
                    &root["ledger"]["pending"],
                    Some(quarantined),
                    epoch,
                )
            }
            Format::FlatV4 => {
                fields(
                    root,
                    &[
                        "version",
                        "available",
                        "pending",
                        "retiring",
                        "replacementReceipts",
                        "usage",
                    ],
                    &["issuance", "scopeEpoch", "scopeKey", "identityKey", "reserveDeposit"],
                )?;
                if root["version"] != 4 {
                    return Err(Error::StorageInvalid);
                }
                let epoch = root
                    .get("scopeEpoch")
                    .map(flat_epoch)
                    .transpose()?
                    .unwrap_or(0);
                (&root["available"], &root["pending"], None, Some(epoch))
            }
        };
        let mut tokens = HashSet::new();
        let mut lease_ids = HashSet::new();
        for pass in list(available, 4096)? {
            self.pass(pass, &mut tokens)?;
        }
        for lease in list(pending, 512)? {
            match self.format {
                Format::LedgerV5 => fields(lease, &["id", "passes"], &[])?,
                Format::AndroidV8 => unreachable!("validated above"),
                Format::FlatV4 => fields(
                    lease,
                    &["id", "passes"],
                    &["createdAt", "scopeEpoch", "usageDay"],
                )?,
            }
            let id = text(lease, "id", 128)?;
            if !lease_ids.insert(id) {
                return Err(Error::StorageInvalid);
            }
            if self.format == Format::FlatV4 {
                if let Some(value) = lease.get("scopeEpoch") {
                    flat_epoch(value)?;
                }
                if let Some(value) = lease.get("createdAt") {
                    crate_date(value)?;
                }
                if let Some(day) = lease.get("usageDay").filter(|v| !v.is_null()) {
                    valid_day(day)?;
                }
            }
            let passes = list(&lease["passes"], crate::style_inventory::BATCH)?;
            if passes.is_empty() {
                return Err(Error::StorageInvalid);
            }
            for pass in passes {
                self.pass(pass, &mut tokens)?;
            }
        }
        if let Some(quarantined) = quarantined {
            let mut ids = HashSet::new();
            for value in quarantined {
                let id = value.as_str().ok_or(Error::StorageInvalid)?;
                if !ids.insert(id) || !lease_ids.contains(id) {
                    return Err(Error::StorageInvalid);
                }
            }
            if epoch.is_none() && ids.len() != lease_ids.len() {
                return Err(Error::StorageInvalid);
            }
        }
        let limit = if self.format == Format::LedgerV5 {
            256
        } else {
            4096
        };
        for pass in list(&root["retiring"], limit)? {
            self.pass(pass, &mut tokens)?;
        }
        if tokens.len() > 4096 {
            return Err(Error::StorageInvalid);
        }
        let mut receipts = HashSet::new();
        for receipt in list(&root["replacementReceipts"], limit)? {
            self.receipt(receipt, &mut receipts)?;
        }
        if !root["issuance"].is_null() {
            self.journal(&root["issuance"])?;
        }
        super::deposit::validate(root,epoch)?;
        let ledger: crate::usage_ledger::Ledger =
            serde_json::from_value(root["usage"].clone()).map_err(|_| Error::StorageInvalid)?;
        crate::usage_ledger::apply(ledger, crate::usage_ledger::Event::Validate)
            .map_err(|_| Error::StorageInvalid)?;
        Ok(())
    }
    pub(super) fn receipt(&self, receipt: &Value, tokens: &mut HashSet<String>) -> Result<()> {
        fields(receipt, &["token", "receivedAt"], &["usageScopeId"])?;
        if !receipt["usageScopeId"].is_null() && !uuid(&receipt["usageScopeId"]) {return Err(Error::StorageInvalid);}
        let token = text(receipt, "token", 4096)?;
        if token.len() < 64 || !tokens.insert(token) {
            return Err(Error::StorageInvalid);
        }
        self.format.date_millis(&receipt["receivedAt"])?;
        Ok(())
    }
    pub(super) fn pass(&self, pass: &Value, tokens: &mut HashSet<String>) -> Result<()> {
        let key = self.format.key_id_field();
        fields(
            pass,
            &["token", key, "expiresAt", "expiresWithScope"],
            &["usageScopeId"],
        )?;
        let token = text(pass, "token", 16384)?;
        if !tokens.insert(token)
            || !valid_key(&text(pass, key, 64)?)
            || pass["expiresWithScope"].as_bool().is_none()
        {
            return Err(Error::StorageInvalid);
        }
        self.format.date_millis(&pass["expiresAt"])?;
        if let Some(value) = pass.get("usageScopeId").filter(|v| !v.is_null()) {
            if !uuid(value) {
                return Err(Error::StorageInvalid);
            }
        }
        Ok(())
    }
    pub(super) fn journal(&self, journal: &Value) -> Result<()> {
        if self.format == Format::AndroidV8 {
            return super::android_v8::validate_issuance(journal);
        }
        let key = self.format.key_id_field();
        fields(
            journal,
            &["idempotencyKey", key, "expiresAt", "requests"],
            &[
                "scopeEpoch",
                "expiresWithScope",
                "replacementReceipts",
                "replayUntil",
            ],
        )?;
        if !uuid(&journal["idempotencyKey"]) || !valid_key(&text(journal, key, 64)?) {
            return Err(Error::StorageInvalid);
        }
        let expiry = self.format.date_millis(&journal["expiresAt"])?;
        // A persisted replay deadline is required for the current producer.
        self.format.date_millis(&journal["replayUntil"])?;
        if let Some(value) = journal.get("scopeEpoch") {
            if self.format == Format::FlatV4 {
                flat_epoch(value)?;
            } else {
                optional_epoch(value)?;
            }
        }
        if journal
            .get("expiresWithScope")
            .is_some_and(|v| v.as_bool().is_none())
        {
            return Err(Error::StorageInvalid);
        }
        let requests = list(&journal["requests"], crate::style_inventory::BATCH)?;
        if requests.is_empty() {
            return Err(Error::StorageInvalid);
        }
        let mut receipts = HashSet::new();
        if let Some(values) = journal.get("replacementReceipts") {
            for value in list(values, requests.len())? {
                let token = value
                    .as_str()
                    .filter(|s| (64..=4096).contains(&s.len()))
                    .ok_or(Error::StorageInvalid)?;
                if !receipts.insert(token) {
                    return Err(Error::StorageInvalid);
                }
            }
        }
        for raw in requests {
            let request = self.format.request(raw)?;
            if request.key_id != journal[key].as_str().ok_or(Error::StorageInvalid)?
                || request.key_not_after_millis < expiry
            {
                return Err(Error::StorageInvalid);
            }
        }
        Ok(())
    }
}
pub(super) fn list(value: &Value, max: usize) -> Result<&[Value]> {
    value
        .as_array()
        .filter(|a| a.len() <= max)
        .map(Vec::as_slice)
        .ok_or(Error::StorageInvalid)
}
fn flat_epoch(value: &Value) -> Result<u64> {
    optional_epoch(value)?.ok_or(Error::StorageInvalid)
}
pub(super) fn optional_epoch(value: &Value) -> Result<Option<u64>> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_u64()
        .filter(|n| *n <= i64::MAX as u64)
        .map(Some)
        .ok_or(Error::StorageInvalid)
}
fn valid_key(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn uuid(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == 36
            && s.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
    })
}
fn crate_date(value: &Value) -> Result<()> {
    value
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|_| ())
        .ok_or(Error::StorageInvalid)
}
fn valid_day(value: &Value) -> Result<()> {
    let day = value.as_str().ok_or(Error::StorageInvalid)?;
    if chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok() && day.len() == 10 {
        Ok(())
    } else {
        Err(Error::StorageInvalid)
    }
}
