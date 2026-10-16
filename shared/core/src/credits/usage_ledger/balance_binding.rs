//! Current profile/stored balance codec. Shared ledger validation owns its caps.
use super::{validation,Days,RequestCount,Scope};
use crate::{Error,Result};
use chrono::{DateTime,Duration};
use serde_json::{json,Value};
use std::collections::BTreeMap;
fn balance_scope(v:&Value)->Result<Scope> {
    serde_json::from_value(json!({"scopeId":v["scopeId"],"day":v["day"],
        "settlementScopeIds":v.get("settlementScopeIds").cloned().unwrap_or(json!([])),
        "dailySettlementScopeIds":v.get("dailySettlementScopeIds").cloned().unwrap_or(json!([]))})).map_err(|_|Error::InvalidResponse)
}
fn aliases(v:&Value)->Result<()> {
    let full:Vec<String>=serde_json::from_value(v.get("settlementScopeIds").cloned().unwrap_or(json!([]))).map_err(|_|Error::InvalidResponse)?;
    let daily:Vec<String>=serde_json::from_value(v.get("dailySettlementScopeIds").cloned().unwrap_or(json!([]))).map_err(|_|Error::InvalidResponse)?;
    validation::aliases(v["scopeId"].as_str().unwrap_or(""),&full,&daily)?;
    if full.iter().chain(&daily).any(|id|id.bytes().any(|b|b.is_ascii_uppercase())) {return Err(Error::InvalidResponse);}
    Ok(())
}
pub(crate) fn evaluate(c:Value)->Result<Value> {
    let v=&c["balance"];
    if c["kind"]=="fresh" {
        let fresh=v["serverTime"].as_str().and_then(|s|DateTime::parse_from_rfc3339(s).ok())
            .zip(c["elapsedNanos"].as_i64()).and_then(|(date,nanos)|date.checked_add_signed(Duration::nanoseconds(nanos.max(0))))
            .is_some_and(|date|date.with_timezone(&chrono::Utc).date_naive().to_string()==v["day"].as_str().unwrap_or(""));
        return Ok(json!({"fresh":fresh}));
    }
    aliases(v)?;
    if c["kind"]=="aliases" {return Ok(json!({}));}
    validation::scope(&balance_scope(v)?)?;
    if v["version"]!=1 || DateTime::parse_from_rfc3339(v["serverTime"].as_str().ok_or(Error::InvalidResponse)?).is_err() {return Err(Error::InvalidResponse);}
    for k in ["revision","consumedToday","consumedTotal","remainingToday","remainingTotal"] {
        if v[k].as_i64().is_none_or(|n|n<0 || (k!="revision" && n>i32::MAX as i64)) {return Err(Error::InvalidResponse);}
    }
    if v["consumedTotal"].as_i64()<v["consumedToday"].as_i64() && v["dailySettlementScopeIds"].as_array().is_none_or(|a|a.is_empty()) {return Err(Error::InvalidResponse);}
    let days:Days=serde_json::from_value(v.get("deviceDays").cloned().unwrap_or(json!({}))).map_err(|_|Error::InvalidResponse)?;
    let requests:BTreeMap<String,RequestCount>=serde_json::from_value(v.get("deviceRequests").cloned().unwrap_or(json!({}))).map_err(|_|Error::InvalidResponse)?;
    validation::days(&days)?;validation::requests(&requests,&days)?;
    Ok(json!({"balance":v}))
}

/// A confirmed balance cannot be rolled back by a delayed full profile or issue reply.
pub(crate) fn older(incoming:&Value,confirmed:&Value)->bool {
    incoming.is_object() && confirmed.is_object() && incoming["scopeId"]==confirmed["scopeId"] && {
        let day=incoming["day"].as_str().unwrap_or("");let current=confirmed["day"].as_str().unwrap_or("");
        day<current || day==current && incoming["revision"].as_u64().unwrap_or(0)<confirmed["revision"].as_u64().unwrap_or(0)
    }
}
