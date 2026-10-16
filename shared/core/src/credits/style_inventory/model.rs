use serde::{Deserialize, Serialize};

/// IDs reference objects held by the native secure-storage port. Neither signed
/// passes nor unblinding material crosses this business boundary.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pass {
    pub id: String,
    pub expires_at_millis: u64,
    pub expires_with_scope: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lease {
    pub id: String,
    pub passes: Vec<Pass>,
    pub scope_epoch: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub id: String,
    pub received_at_millis: u64,
    pub group: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Journal {
    pub id: String,
    pub scope_epoch: Option<u64>,
    pub count: usize,
    pub receipts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub available: Vec<Pass>,
    pub pending: Vec<Lease>,
    pub retiring: Vec<Pass>,
    pub receipts: Vec<Receipt>,
    pub issuance: Option<Journal>,
    pub scope_epoch: Option<u64>,
    pub deposit_receipts: Vec<String>,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    Normalize,
    ChangeScope {
        compatible: bool,
    },
    Lease {
        id: String,
        count: usize,
    },
    Settle {
        id: String,
        restore: bool,
    },
    BeginIssuance {
        journal: Journal,
    },
    FinishIssuance {
        id: String,
        passes: Vec<Pass>,
    },
    DiscardIssuance {
        id: String,
        scope_epoch: Option<u64>,
        discard_receipts: bool,
    },
    SelectReceipts {
        count: usize,
    },
    Retire {
        scope_epoch: u64,
        selected: Vec<String>,
        receipts: Vec<Receipt>,
        discard: bool,
    },
    ReturnUnused,
    SelectDeposit,
    CompleteDeposit {
        scope_epoch: u64,
        receipts: Vec<String>,
    },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub state: State,
    pub selected: Vec<String>,
    pub retirement: Vec<String>,
    pub applied: bool,
    pub merged: bool,
    pub ready: usize,
    pub pending: usize,
    pub issuance_pending: bool,
    pub reserve_ready: bool,
}
