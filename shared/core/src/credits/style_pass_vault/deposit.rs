//! The exact receipt journal survives business-scope changes, never a future epoch.
use super::{envelope::{list,optional_epoch},format::{fields,text}};
use crate::{Error,Result};
use serde_json::Value;
use std::collections::HashSet;
pub(super) fn validate(root:&Value,epoch:Option<u64>)->Result<()> {
    let deposit=&root["reserveDeposit"];
    if deposit.is_null() {return Ok(());}
    fields(deposit,&["id","scopeId","scopeEpoch","receipts"],&[])?;
    let id=text(deposit,"id",128)?;let scope=text(deposit,"scopeId",128)?;
    let deposited=optional_epoch(&deposit["scopeEpoch"])?;
    if !crate::request::uuid(&id) || id.bytes().any(|c|c.is_ascii_uppercase())
        || !crate::request::uuid(&scope) || deposited.is_none()
        || deposited.zip(epoch).is_none_or(|(old,current)|old>current) {return Err(Error::StorageInvalid);}
    let selected=list(&deposit["receipts"],crate::style_inventory::BATCH)?;
    if selected.is_empty() {return Err(Error::StorageInvalid);}
    let receipts=root["replacementReceipts"].as_array().ok_or(Error::StorageInvalid)?;
    let mut seen=HashSet::new();
    for token in selected {
        if !token.is_string() || !seen.insert(token)
            || !receipts.iter().any(|r|r["token"]==*token && r["usageScopeId"]==deposit["scopeId"])
            || root["issuance"]["replacementReceipts"].as_array().is_some_and(|rs|rs.contains(token)) {
            return Err(Error::StorageInvalid);
        }
    }
    Ok(())
}
