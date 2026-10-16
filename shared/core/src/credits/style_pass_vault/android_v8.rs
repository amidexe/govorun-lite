//! Android's current protected v8 envelope. Validate stored pass state before DTO mapping;
//! retain its field names, seconds, caps and three-field issuance material.
use super::format::{fields, text};
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashSet;

fn check(value: bool) -> Result<()> { if value { Ok(()) } else { Err(Error::StorageInvalid) } }
fn list(value: &Value, max: usize) -> Result<&Vec<Value>> {
    value.as_array().filter(|v|v.len()<=max).ok_or(Error::StorageInvalid)
}
fn number(value: &Value, positive: bool) -> Result<i64> {
    value.as_i64().filter(|n|if positive {*n>0} else {*n>=0}).ok_or(Error::StorageInvalid)
}
fn uuid(value: &str, lowercase: bool) -> Result<()> {
    check(crate::request::uuid(value) && (!lowercase || !value.bytes().any(|c|c.is_ascii_uppercase())))
}
fn optional_uuid(value: &Value) -> Result<()> {
    if !value.is_null() { uuid(value.as_str().ok_or(Error::StorageInvalid)?,false)?; } Ok(())
}
fn pass(value: &Value, seen: &mut HashSet<String>) -> Result<()> {
    fields(value,&["token","keyId","expiresAt","expiresWithScope"],&["usageScopeId"])?;
    check(seen.insert(text(value,"token",8*1024*1024)?))?;
    text(value,"keyId",8*1024*1024)?; number(&value["expiresAt"],true)?;
    check(value["expiresWithScope"].is_boolean())?; optional_uuid(&value["usageScopeId"])
}

pub(crate) fn validate(root: Value) -> Result<Value> {
    super::binding::validate_keys(&root)?;
    fields(&root,&["version","scopeEpoch","usage","available","pending","retiring","replacementReceipts"],
        &["issuance","reserveDeposit","identityKey","scopeKey"])?;
    check(root["version"]==8)?;
    let epoch=number(&root["scopeEpoch"],false)?;
    fields(&root["usage"],&["totals","acknowledged"],&["requests"])?;
    let ledger=crate::usage_ledger::apply(serde_json::from_value(root["usage"].clone())
        .map_err(|_|Error::StorageInvalid)?,crate::usage_ledger::Event::Validate)
        .map_err(|_|Error::StorageInvalid)?.ledger;
    let mut tokens=HashSet::new(); let mut ids=HashSet::new();
    for name in ["available","retiring"] { for item in list(&root[name],256)? { pass(item,&mut tokens)?; } }
    for lease in list(&root["pending"],64)? {
        fields(lease,&["id","scopeEpoch","passes"],&["usageDay"])?;
        let id=text(lease,"id",128)?; uuid(&id,true)?; check(ids.insert(id))?;
        check(number(&lease["scopeEpoch"],false)?<=epoch)?;
        if !lease["usageDay"].is_null() {
            let day=lease["usageDay"].as_str().ok_or(Error::StorageInvalid)?;
            crate::usage_ledger::validation::day(day).map_err(|_|Error::StorageInvalid)?;
        }
        let passes=list(&lease["passes"],crate::style_inventory::BATCH)?; check(!passes.is_empty())?;
        for item in passes { pass(item,&mut tokens)?; }
    }
    let receipts=list(&root["replacementReceipts"],256)?; let mut receipt_tokens=HashSet::new();
    for receipt in receipts {
        fields(receipt,&["token","receivedAt"],&["usageScopeId"])?;
        check(receipt_tokens.insert(text(receipt,"token",8*1024*1024)?))?;
        number(&receipt["receivedAt"],true)?; optional_uuid(&receipt["usageScopeId"])?;
    }
    super::deposit::validate(&root,Some(epoch as u64))?;
    if !root["issuance"].is_null() {
        validate_issuance(&root["issuance"])?;
        check(number(&root["issuance"]["scopeEpoch"],false)?<=epoch)?;
    }
    Ok(json!({"usage":ledger}))
}

/// The current journal format retains its key and the three RFC blinding fields.
pub(super) fn validate_issuance(issue: &Value) -> Result<()> {

        fields(issue,&["id","keyId","algorithm","publicKey","notBefore","notAfter","expiresAt","replayUntil",
            "expiresWithScope","scopeEpoch","requests","replacementReceipts"],&[])?;
        uuid(&text(issue,"id",128)?,true)?;
        for name in ["keyId","algorithm","publicKey","notBefore","notAfter"] { text(issue,name,8*1024*1024)?; }
        number(&issue["expiresAt"],true)?; number(&issue["replayUntil"],true)?;
        check(issue["expiresWithScope"].is_boolean() && number(&issue["scopeEpoch"],false)?<=i64::MAX)?;
        let requests=list(&issue["requests"],crate::style_inventory::BATCH)?; check(!requests.is_empty())?;
        for request in requests {
            fields(request,&["preparedMessage","salt","inverse"],&[])?;
            for name in ["preparedMessage","salt","inverse"] { text(request,name,8*1024*1024)?; }
        }
        for receipt in list(&issue["replacementReceipts"],requests.len())? {
            check(receipt.as_str().is_some_and(|s|!s.trim().is_empty()))?;
        }
    Ok(())
}
