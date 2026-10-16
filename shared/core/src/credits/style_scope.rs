//! Local access-period and issuance-admission facts. These views never
//! replace the live account/session validation at the network boundary.
use crate::Result;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scope {
    account_id: Option<String>, signed_in: bool, generation: u64,
    terms: u32, consent_version: u32, consent_active: bool, access: Value,
}
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Command {
    Controls { facts: Value },
    Facts { access: Value },
    Period { first: Value, second: Value },
    Match { scope: Scope, current: Scope },
    Prewire { same_scope: bool },
    RefreshFailure { status: Option<u16> },
    Enabled { self_hosted: bool, enabled: bool, processing_ready: bool, protocol_supported: bool, consent: bool },
}
fn same_period(first: &Value, second: &Value) -> bool {
    ["type","startsAt","endsAt","planCode"].iter().all(|name| first[*name]==second[*name])
}
pub(crate) fn evaluate(command: Command) -> Result<Value> {
    Ok(match command {
        Command::Controls { facts:f } => {
            let signed=f["signedIn"]==true;let entitled=signed && f["entitlementActive"]==true;let authorized=signed && f["authorized"]==true;
            json!({"entitlement":entitled,"authorized":authorized,"processingReady":signed && f["storageAvailable"]==true && f["processingReady"]==true,
                "enableAction":if f["enableAllowed"]==true {"CONTINUE"} else {"BLOCK_WITH_PASSIVE_NOTICE"},
                "termsRequired":f["termsActionAvailable"]==true && (f["localCurrentTermsAccepted"]!=true || f["consentAccepted"]!=true),
                "active":f["enabled"]==true && f["localConsent"]==true && authorized,
                "selected":f["enabled"]==true && signed && (entitled || f["displayEntitlementActive"]==true)})
        },
        Command::Facts { access } => json!({"protocolSupported":crate::authority::protocol_supported(&access),
            "reserveReturn":access["reserveReturnProtocol"]=="v1_receipt_deposit"}),
        Command::Period { first,second } => json!({"value":same_period(&first,&second)}),
        Command::Match { scope,current } => json!({"value":current.account_id==scope.account_id && current.signed_in
            && current.generation==scope.generation && same_period(&current.access,&scope.access)
            && current.terms==scope.terms && current.consent_version==scope.consent_version && current.consent_active==scope.consent_active}),
        Command::Prewire { same_scope } => json!({"settlement":if same_scope {"ready"} else {"discarded"}}),
        Command::RefreshFailure { status } => json!({"value":status==Some(429)}),
        Command::Enabled { self_hosted,enabled,processing_ready,protocol_supported,consent } =>
            json!({"value":!self_hosted && enabled && processing_ready && protocol_supported && consent}),
    })
}
