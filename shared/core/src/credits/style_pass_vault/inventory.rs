//! Retains stored token objects verbatim while the existing common inventory
//! decides transitions. No signature, nonce or token is a published view.
use super::{
    envelope::{list, optional_epoch},
    format::{text, Format},
    Envelope,
};
use crate::{style_inventory as inventory, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Additions {
    #[serde(default)]
    pub passes: Vec<Value>,
    #[serde(default)]
    pub receipts: Vec<Value>,
    pub journal: Option<Value>,
}
pub struct InventoryChange {
    pub candidate: Envelope,
    pub selected: Vec<String>,
    pub retirement: Vec<String>,
    pub ready: usize,
    pub pending: usize,
    pub issuance_pending: bool,
    pub applied: bool,
    pub merged: bool,
}
pub(super) struct Index {
    passes: BTreeMap<String, Value>,
    pass_ids: BTreeMap<String, String>,
    receipts: BTreeMap<String, Value>,
    receipt_ids: BTreeMap<String, String>,
}
impl Index {
    pub(super) fn for_envelope(vault: &Envelope, additions: &Additions) -> Result<Self> {
        let root = &vault.source;
        let ledger = if vault.format == Format::LedgerV5 {
            &root["ledger"]
        } else {
            root
        };
        let mut index = Self::new();
        for pass in list(&ledger["available"], 4096)?
            .iter()
            .chain(
                list(&ledger["pending"], 512)?
                    .iter()
                    .flat_map(|l| l["passes"].as_array().into_iter().flatten()),
            )
            .chain(list(&root["retiring"], 4096)?)
            .chain(&additions.passes)
        {
            index.add(pass, true)?;
        }
        for receipt in list(&root["replacementReceipts"], 4096)?
            .iter()
            .chain(&additions.receipts)
        {
            index.add(receipt, false)?;
        }
        for journal in [root.get("issuance"), additions.journal.as_ref()]
            .into_iter()
            .flatten()
            .filter(|v| !v.is_null())
        {
            vault.journal(journal)?;
            for token in journal
                .get("replacementReceipts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let token = token.as_str().ok_or(Error::StorageInvalid)?;
                let next = format!("r{}", index.receipt_ids.len());
                index.receipt_ids.entry(token.into()).or_insert(next);
            }
        }
        Ok(index)
    }
    pub(super) fn pass_id(&self, token: &str) -> Result<String> {
        self.pass_ids
            .get(token)
            .cloned()
            .ok_or(Error::InvalidRequest)
    }
    pub(super) fn receipt_id(&self, token: &str) -> Result<String> {
        self.receipt_ids
            .get(token)
            .cloned()
            .ok_or(Error::InvalidRequest)
    }
    fn new() -> Self {
        Self {
            passes: BTreeMap::new(),
            pass_ids: BTreeMap::new(),
            receipts: BTreeMap::new(),
            receipt_ids: BTreeMap::new(),
        }
    }
    fn add(&mut self, value: &Value, pass: bool) -> Result<()> {
        let max = if pass { 16384 } else { 4096 };
        let token = text(value, "token", max)?;
        let (ids, values, prefix) = if pass {
            (&mut self.pass_ids, &mut self.passes, "p")
        } else {
            (&mut self.receipt_ids, &mut self.receipts, "r")
        };
        let next = format!("{prefix}{}", ids.len());
        let id = ids.entry(token).or_insert(next).clone();
        values.insert(id, value.clone());
        Ok(())
    }
    pub(super) fn pass(&self, value: &Value, format: Format) -> Result<inventory::Pass> {
        Ok(inventory::Pass {
            id: self
                .pass_ids
                .get(&text(value, "token", 16384)?)
                .cloned()
                .ok_or(Error::StorageInvalid)?,
            expires_at_millis: format.date_millis(&value["expiresAt"])?,
            expires_with_scope: value["expiresWithScope"]
                .as_bool()
                .ok_or(Error::StorageInvalid)?,
        })
    }
    pub(super) fn receipt(&self, value: &Value, format: Format) -> Result<inventory::Receipt> {
        Ok(inventory::Receipt {
            id: self
                .receipt_ids
                .get(&text(value, "token", 4096)?)
                .cloned()
                .ok_or(Error::StorageInvalid)?,
            received_at_millis: format.date_millis(&value["receivedAt"])?,
            group: value["usageScopeId"].as_str().map(str::to_owned),
        })
    }
    fn values(&self, passes: &[inventory::Pass]) -> Result<Vec<Value>> {
        passes
            .iter()
            .map(|p| self.passes.get(&p.id).cloned().ok_or(Error::StorageInvalid))
            .collect()
    }
    fn tokens(&self, ids: &[String]) -> Result<Vec<String>> {
        ids.iter()
            .map(|id| {
                self.passes
                    .get(id)
                    .or_else(|| self.receipts.get(id))
                    .and_then(|v| v["token"].as_str())
                    .map(str::to_owned)
                    .ok_or(Error::StorageInvalid)
            })
            .collect()
    }
    pub(super) fn journal(&self, value: &Value, epoch: Option<u64>) -> Result<inventory::Journal> {
        let receipts = value
            .get("replacementReceipts")
            .map(|v| list(v, crate::style_inventory::BATCH))
            .transpose()?
            .unwrap_or(&[]);
        Ok(inventory::Journal {
            id: text(value, if value.get("id").is_some() {"id"} else {"idempotencyKey"}, 128)?,
            scope_epoch: epoch,
            count: list(&value["requests"], crate::style_inventory::BATCH)?.len(),
            receipts: receipts
                .iter()
                .map(|r| {
                    r.as_str()
                        .and_then(|t| self.receipt_ids.get(t))
                        .cloned()
                        .ok_or(Error::StorageInvalid)
                })
                .collect::<Result<_>>()?,
        })
    }
}
impl Envelope {
    pub fn inventory(
        &self,
        additions: Additions,
        event: inventory::Event,
        now: u64,
        retirement_available: bool,
    ) -> Result<InventoryChange> {
        let f = self.format;
        let root = &self.source;
        let ledger = if f == Format::LedgerV5 {
            &root["ledger"]
        } else {
            root
        };
        let available = list(&ledger["available"], 4096)?;
        let pending = list(&ledger["pending"], 512)?;
        let retiring = list(&root["retiring"], 4096)?;
        let receipts = list(&root["replacementReceipts"], 4096)?;
        let index = Index::for_envelope(self, &additions)?;
        let mut used = HashSet::new();
        for pass in &additions.passes {
            self.pass(pass, &mut used)?;
        }
        let mut used_receipts = HashSet::new();
        for receipt in &additions.receipts {
            self.receipt(receipt, &mut used_receipts)?;
        }
        let epoch = if f != Format::LedgerV5 {
            Some(optional_epoch(&root["scopeEpoch"])?.unwrap_or(0))
        } else {
            optional_epoch(&root["scopeEpoch"])?
        };
        let pass_list = |values: &[Value]| {
            values
                .iter()
                .map(|p| index.pass(p, f))
                .collect::<Result<Vec<_>>>()
        };
        let quarantined = root["quarantinedPendingLeaseIDs"].as_array();
        let pending_state = pending
            .iter()
            .map(|p| {
                Ok(inventory::Lease {
                    id: text(p, "id", 128)?,
                    passes: pass_list(list(&p["passes"], crate::style_inventory::BATCH)?)?,
                    scope_epoch: if f != Format::LedgerV5 {
                        Some(optional_epoch(&p["scopeEpoch"])?.unwrap_or(0))
                    } else if quarantined.is_some_and(|q| q.contains(&p["id"])) {
                        None
                    } else {
                        epoch
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let journal = root
            .get("issuance")
            .filter(|v| !v.is_null())
            .map(|j| {
                index.journal(
                    j,
                    optional_epoch(if f == Format::LedgerV5 {
                        &root["issuanceScopeEpoch"]
                    } else {
                        &j["scopeEpoch"]
                    })?,
                )
            })
            .transpose()?;
        let state = inventory::State {
            available: pass_list(available)?,
            pending: pending_state,
            retiring: pass_list(retiring)?,
            receipts: receipts
                .iter()
                .map(|r| index.receipt(r, f))
                .collect::<Result<_>>()?,
            issuance: journal,
            scope_epoch: epoch,
            deposit_receipts: root["reserveDeposit"]["receipts"].as_array().into_iter().flatten()
                .map(|v|v.as_str().ok_or(Error::StorageInvalid).and_then(|t|index.receipt_id(t))).collect::<Result<_>>()?,
        };
        let previous = state.clone();
        let decision = inventory::transition(state, event, now, retirement_available)?;
        if decision.state == previous
            && additions.passes.is_empty()
            && additions.receipts.is_empty()
            && additions.journal.is_none()
        {
            return Ok(InventoryChange {
                candidate: self.clone(),
                selected: index.tokens(&decision.selected)?,
                retirement: index.tokens(&decision.retirement)?,
                ready: decision.ready,
                pending: decision.pending,
                issuance_pending: decision.issuance_pending,
                applied: decision.applied,
                merged: decision.merged,
            });
        }
        let mut output = root.clone();
        let out_ledger = if f == Format::LedgerV5 {
            &mut output["ledger"]
        } else {
            &mut output
        };
        out_ledger["available"] = json!(index.values(&decision.state.available)?);
        let stored_pending = decision
            .state
            .pending
            .iter()
            .map(|l| {
                let previous = pending
                    .iter()
                    .find(|p| p["id"].as_str() == Some(l.id.as_str()));
                let mut stored = previous.cloned().unwrap_or_else(|| json!({"id":l.id}));
                stored["passes"] = json!(index.values(&l.passes)?);
                if f != Format::LedgerV5 {
                    stored["scopeEpoch"] = json!(l.scope_epoch);
                    if previous.is_none() && f == Format::FlatV4 {
                        stored["createdAt"] = f.date_value(now)?;
                    }
                }
                Ok(stored)
            })
            .collect::<Result<Vec<Value>>>()?;
        out_ledger["pending"] = json!(stored_pending);
        output["scopeEpoch"] = json!(decision.state.scope_epoch);
        output["retiring"] = json!(index.values(&decision.state.retiring)?);
        output["replacementReceipts"] = json!(decision
            .state
            .receipts
            .iter()
            .map(|r| index
                .receipts
                .get(&r.id)
                .cloned()
                .ok_or(Error::StorageInvalid))
            .collect::<Result<Vec<_>>>()?);
        let selected_journal = decision
            .state
            .issuance
            .as_ref()
            .map(|j| {
                [root.get("issuance"), additions.journal.as_ref()]
                    .into_iter()
                    .flatten()
                    .find(|v| v[f.journal_id_field()].as_str() == Some(j.id.as_str()))
                    .cloned()
                    .ok_or(Error::StorageInvalid)
            })
            .transpose()?;
        if decision.state.deposit_receipts.is_empty() {
            output.as_object_mut().ok_or(Error::StorageInvalid)?.remove("reserveDeposit");
        }
        output["issuance"] = selected_journal.clone().unwrap_or(Value::Null);
        if f == Format::LedgerV5 {
            output["issuanceScopeEpoch"] =
                json!(decision.state.issuance.as_ref().and_then(|j| j.scope_epoch));
            output["quarantinedPendingLeaseIDs"] = json!(decision
                .state
                .pending
                .iter()
                .filter(|l| l.scope_epoch != decision.state.scope_epoch || l.scope_epoch.is_none())
                .map(|l| l.id.clone())
                .collect::<Vec<_>>());
        }
        let candidate = Envelope::decode(&output.to_string(), f)?;
        Ok(InventoryChange {
            candidate,
            selected: index.tokens(&decision.selected)?,
            retirement: index.tokens(&decision.retirement)?,
            ready: decision.ready,
            pending: decision.pending,
            issuance_pending: decision.issuance_pending,
            applied: decision.applied,
            merged: decision.merged,
        })
    }
}
