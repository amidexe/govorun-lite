//! Host-only protected mutations. Tokens are mapped to private inventory IDs;
//! no caller needs to know the temporary index or native layout.
use super::{format::Format, inventory::Index, Additions, Envelope, InventoryChange};
use crate::{style_inventory as inventory, usage_ledger as usage, Error, Result};
use serde_json::{json, Value};

pub enum Action {
    Inventory(inventory::Event),
    Lease {
        id: String,
        count: usize,
        day: Option<String>,
        admitted_scope: Option<String>,
    },
    Settle {
        id: String,
        restore: bool,
        consumed: bool,
    },
    BeginIssuance {
        journal: Value,
        expected_scope_epoch: u64,
    },
    FinishIssuance {
        id: String,
        passes: Vec<Value>,
    },
    Retire {
        scope_epoch: u64,
        tokens: Vec<String>,
        receipts: Vec<Value>,
        discard: bool,
    },
    CompleteDeposit {
        scope_epoch: u64,
        receipts: Vec<String>,
    },
    BeginDeposit { id: String },
    Usage(usage::Event),
}
impl Envelope {
    pub fn change(
        &self,
        action: Action,
        now: u64,
        retirement_available: bool,
    ) -> Result<InventoryChange> {
        let mut additions = Additions::default();
        let mut begin = None;
        let mut settle = None;
        let mut usage_event = None;
        let mut deposit = None;
        let event = match action {
            Action::Inventory(e) => e,
            Action::Lease {
                id,
                count,
                day,
                admitted_scope,
            } => {
                begin = Some((id.clone(), day, admitted_scope));
                inventory::Event::Lease { id, count }
            }
            Action::Settle {
                id,
                restore,
                consumed,
            } => {
                if restore && consumed {
                    return Err(Error::InvalidRequest);
                }
                settle = Some((id.clone(), consumed));
                inventory::Event::Settle { id, restore }
            }
            Action::BeginIssuance {
                mut journal,
                expected_scope_epoch,
            } => {
                if self.scope_epoch()? != Some(expected_scope_epoch) {
                    return Err(Error::SessionChanged);
                }
                if self.format != Format::LedgerV5 {
                    journal["scopeEpoch"] = json!(expected_scope_epoch);
                }
                additions.journal = Some(journal);
                let index = Index::for_envelope(self, &additions)?;
                let journal =
                    index.journal(additions.journal.as_ref().unwrap(), self.scope_epoch()?)?;
                inventory::Event::BeginIssuance { journal }
            }
            Action::FinishIssuance { id, passes } => {
                additions.passes = passes;
                let index = Index::for_envelope(self, &additions)?;
                let passes = additions
                    .passes
                    .iter()
                    .map(|p| index.pass(p, self.format))
                    .collect::<Result<_>>()?;
                inventory::Event::FinishIssuance { id, passes }
            }
            Action::Retire {
                scope_epoch,
                tokens,
                receipts,
                discard,
            } => {
                additions.receipts = receipts;
                let index = Index::for_envelope(self, &additions)?;
                let selected = tokens
                    .iter()
                    .map(|token| index.pass_id(token))
                    .collect::<Result<_>>()?;
                let receipts = additions
                    .receipts
                    .iter()
                    .map(|r| index.receipt(r, self.format))
                    .collect::<Result<_>>()?;
                inventory::Event::Retire {
                    scope_epoch,
                    selected,
                    receipts,
                    discard,
                }
            }
            Action::CompleteDeposit {
                scope_epoch,
                receipts,
            } => {
                // A duplicate or late ACK owns only its exact receipt journal.
                // Its receipts may already be removed by another exact replay.
                if self.source["reserveDeposit"]["receipts"] != json!(receipts) {
                    inventory::Event::Normalize
                } else {
                    if self.source["reserveDeposit"]["scopeEpoch"].as_u64() != Some(scope_epoch) {
                        return Err(Error::SessionChanged);
                    }
                    let index = Index::for_envelope(self, &additions)?;
                    let receipts = receipts
                        .iter()
                        .map(|token| index.receipt_id(token))
                        .collect::<Result<_>>()?;
                    inventory::Event::CompleteDeposit {
                        scope_epoch,
                        receipts,
                    }
                }
            }
            Action::BeginDeposit { id } => {
                if !crate::request::uuid(&id) || id.bytes().any(|c|c.is_ascii_uppercase()) {
                    return Err(Error::InvalidRequest);
                }
                if self.source["reserveDeposit"].is_null() {
                    deposit = Some(id);
                    inventory::Event::SelectDeposit
                } else { inventory::Event::Normalize }
            }
            Action::Usage(e) => {
                usage_event = Some(e);
                inventory::Event::Normalize
            }
        };
        let mut result = self.inventory(additions, event, now, retirement_available)?;
        if let Some(id) = deposit.filter(|_|!result.selected.is_empty()) {
            let receipts = result.candidate.source["replacementReceipts"].as_array().ok_or(Error::StorageInvalid)?;
            let scope = receipts.iter().find(|r|r["token"].as_str() == result.selected.first().map(String::as_str))
                .and_then(|r|r["usageScopeId"].as_str()).ok_or(Error::StorageInvalid)?.to_owned();
            result.candidate.source["reserveDeposit"] = json!({"id":id,"scopeId":scope,
                "scopeEpoch":result.candidate.scope_epoch()?,"receipts":result.selected});
        }
        if result.applied {
            let mut ledger = self.usage()?;
            if let Some((id, day, admitted_scope)) = begin {
                let lease = result
                    .candidate
                    .stored_leases()?
                    .iter()
                    .find(|l| l["id"] == id)
                    .ok_or(Error::StorageInvalid)?;
                let scopes = lease["passes"]
                    .as_array()
                    .ok_or(Error::StorageInvalid)?
                    .iter()
                    .map(|p| p["usageScopeId"].as_str().map(str::to_owned))
                    .collect();
                ledger = usage::apply(
                    ledger,
                    usage::Event::Begin {
                        lease_id: id.clone(),
                        day: day.clone(),
                        scopes,
                        admitted_scope: admitted_scope.clone(),
                    },
                )?
                .ledger;
                if self.format != Format::LedgerV5 {
                    let leases = result.candidate.source["pending"]
                        .as_array_mut()
                        .ok_or(Error::StorageInvalid)?;
                    let lease = leases
                        .iter_mut()
                        .find(|l| l["id"] == id)
                        .ok_or(Error::StorageInvalid)?;
                    if let Some(day) = day {
                        lease["usageDay"] = json!(day);
                    }
                    if let Some(scope) = admitted_scope {
                        for p in lease["passes"]
                            .as_array_mut()
                            .ok_or(Error::StorageInvalid)?
                        {
                            p["usageScopeId"] = json!(scope);
                        }
                    }
                }
            }
            if let Some((id, consumed)) = settle {
                ledger = usage::apply(
                    ledger,
                    usage::Event::Settle {
                        lease_id: id,
                        consumed,
                    },
                )?
                .ledger;
            }
            if let Some(mut e) = usage_event {
                if self.format != Format::LedgerV5 {
                    if let usage::Event::Observe {
                        protected_scopes, ..
                    } = &mut e
                    {
                        // Current Windows leases may have a scope but no usage
                        // day. Preserve their acknowledged history exactly as
                        // the previous physical-vault observer did.
                        let mut scopes: std::collections::BTreeSet<String> =
                            protected_scopes.iter().cloned().collect();
                        for lease in result.candidate.stored_leases()? {
                            for pass in lease["passes"].as_array().ok_or(Error::StorageInvalid)? {
                                if let Some(scope) = pass["usageScopeId"].as_str() {
                                    scopes.insert(scope.to_owned());
                                }
                            }
                        }
                        *protected_scopes = scopes.into_iter().collect();
                    }
                }
                ledger = usage::apply(ledger, e)?.ledger;
            }
            result.candidate.store_usage(ledger)?;
        }
        result.candidate = Envelope::decode(&result.candidate.encoded()?, self.format)?;
        Ok(result)
    }
    pub fn scope_epoch(&self) -> Result<Option<u64>> {
        let epoch = super::envelope::optional_epoch(&self.source["scopeEpoch"])?;
        Ok(if self.format != Format::LedgerV5 {
            Some(epoch.unwrap_or(0))
        } else {
            epoch
        })
    }
}
