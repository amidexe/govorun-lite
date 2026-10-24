//! Current Android journal codec at the common diagnostics owner. No I/O or recipient.
use crate::{Error,Result};
use serde_json::{json,Value};
use std::collections::BTreeMap;
const OUTCOMES:&[&str]=&["STYLED","FALLBACK","SKIPPED"];
const REASONS:&[&str]=&["SUCCESS","STREAMING_MODE","CUSTOM_STYLE_DRAFT","STYLIZATION_DISABLED","TERMS_UPDATE_REQUIRED","SERVICE_UNAVAILABLE","ACCESS_INACTIVE","SHORT_PHRASE","TEXT_TOO_LONG","NOT_CONFIGURED","APP_RULE_DISABLED","OFFLINE","CONFIRMED_OFFLINE","NO_CHANGE","STYLE_PASS_RESERVE_EMPTY","STYLE_PASS_ISSUE_FAILED","STYLE_PASS_RECOVERY_PENDING","ACCOUNT_STATE_CHANGED","TRANSPORT_FAILED","REQUEST_FAILED","SERVER_BUSY","SERVER_TIMEOUT","SERVER_UNAVAILABLE","INVALID_RESPONSE","INSERTION_FAILED"];
const LIFECYCLE:&[&str]=&["CHECK_COMPLETED","CHECK_SAMPLES_FAILED","CHECK_MODEL_FAILED","CHECK_RECOGNITION_FAILED","SERVICE_CONNECTED","SERVICE_STOPPED","SERVICE_INTERRUPTED","EXIT_LOW_MEMORY","EXIT_CRASH","EXIT_NATIVE_CRASH","EXIT_ANR","EXIT_SELF","EXIT_SIGNALED","EXIT_INITIALIZATION","EXIT_PERMISSION_CHANGED","EXIT_RESOURCE_USAGE","EXIT_USER_REQUESTED","EXIT_USER_STOPPED","EXIT_DEPENDENCY","EXIT_OTHER","EXIT_FREEZER","EXIT_PACKAGE_CHANGED","EXIT_PACKAGE_UPDATED","EXIT_UNKNOWN","EXIT_HISTORY_UNAVAILABLE","ACCOUNT_ENDED_BY_SERVER","STORAGE_MOVED","STORAGE_LEGACY_REIMPORTED","STORAGE_RESTORED_FROM_BACKUP","STORAGE_DATABASE_REPLACED"];
fn allowed(v:&Value,key:&str,values:&[&str])->bool {v[key].as_str().is_some_and(|s|values.contains(&s))}
fn state(v:&Value)->Option<Value> {
    (allowed(v,"outcome",OUTCOMES) && allowed(v,"reason",REASONS)).then(||json!({"outcome":v["outcome"],"reason":v["reason"]}))
}
fn timing(v:&str)->Option<Value> {
    if let Some(fields)=crate::journal_report::timing::parse(v) {return Some(json!(fields));}
    let tail=v.strip_prefix("self_hosted result=")?;let (result,elapsed)=tail.split_once(" elapsed_ms=")?;
    (matches!(result,"success"|"unchanged"|"auth_failed"|"cancelled"|"failed") && !elapsed.is_empty()
        && elapsed.len()<=9 && elapsed.bytes().all(|b|b.is_ascii_digit()))
        .then(||json!({"result":result,"elapsed_ms":elapsed}))
}
fn filtered_entry(v:&Value)->Option<Value> {
    if v["time"].as_i64()?<=0 {return None;}
    let has=|k:&str|v.get(k).is_some();
    if has("outcome")!=has("reason") {return None;}
    let groups=usize::from(has("lifecycle"))+usize::from(has("styleAccess"))+usize::from(has("outcome")||has("timing"));
    if groups!=1 {return None;}
    let mut out=json!({"time":v["time"]});
    if has("outcome") {let s=state(v)?;out["outcome"]=s["outcome"].clone();out["reason"]=s["reason"].clone();}
    if has("lifecycle") {if !allowed(v,"lifecycle",LIFECYCLE) {return None;}out["lifecycle"]=v["lifecycle"].clone();}
    if has("styleAccess") {if !allowed(v,"styleAccess",&["TOKENS_RECEIVED","TOKEN_REJECTED"]) {return None;}out["styleAccess"]=v["styleAccess"].clone();}
    if has("timing") {timing(v["timing"].as_str()?)?;out["timing"]=v["timing"].clone();} Some(out)
}
fn journal(v:&Value)->Value {
    let entries:Vec<_>=v.as_array().into_iter().flatten().filter_map(filtered_entry).collect();
    json!(entries.iter().skip(entries.len().saturating_sub(crate::journal_report::MAXIMUM)).collect::<Vec<_>>())
}
fn counts(v:&Value)->BTreeMap<String,u32> {
    v.as_object().into_iter().flat_map(|o|o.iter()).filter_map(|(k,n)|Some((k.to_ascii_lowercase(),n.as_u64()?.min(u32::MAX as u64) as u32))).collect()
}
pub(crate) fn evaluate(c:Value)->Result<Value> {
    use crate::stylization::reliability as r;
    match c["kind"].as_str() {
        Some("requestFailure")=>Ok(json!({"reason":match c["family"].as_str().unwrap_or("") {
            "paused"=>"SERVER_UNAVAILABLE","offline"=>"OFFLINE","reserveEmpty"=>"STYLE_PASS_RESERVE_EMPTY",
            "issueFailed"=>"STYLE_PASS_ISSUE_FAILED","recoveryPending"=>"STYLE_PASS_RECOVERY_PENDING",
            "account"=>"ACCOUNT_STATE_CHANGED","ambiguous"|"transport"=>"TRANSPORT_FAILED","deadline"=>"SERVER_TIMEOUT",
            "service"=>match c["code"].as_str().unwrap_or("") {"BUSY"=>"SERVER_BUSY","TIMEOUT"=>"SERVER_TIMEOUT","UNAVAILABLE"=>"SERVER_UNAVAILABLE",_=>"INVALID_RESPONSE"},
            "http"=>match c["status"].as_u64() {Some(429|503)=>"SERVER_BUSY",Some(408|504)=>"SERVER_TIMEOUT",Some(502)=>"SERVER_UNAVAILABLE",_=>"REQUEST_FAILED"},
            _=>"REQUEST_FAILED"}})),
        Some("support")=>Ok(json!({"code":if c["legacy"]==true {crate::journal_report::support::reason(c["value"].as_str().unwrap_or(""))} else {crate::journal_report::support::result(c["value"].as_str().unwrap_or(""))}})),
        Some("taxonomy")=>Ok(r::Counters::taxonomy()),
        Some("presentation")=> {
            let context = c.get("state").filter(|value| !value.is_null());
            if context.is_some_and(|value| state(value).is_none()) {return Err(Error::InvalidRequest);}
            let fields=c["value"].as_str().and_then(crate::journal_report::timing::parse);
            if !c["value"].is_null() && fields.is_none() {return Err(Error::InvalidRequest);}
            Ok(crate::journal_report::android_presentation::present(fields,
                context.and_then(|s|s["outcome"].as_str()),context.and_then(|s|s["reason"].as_str())))
        },
        Some("timing")=> {
            let fields=c["value"].as_str().and_then(crate::journal_report::timing::parse);
            Ok(json!({"fields":c["value"].as_str().and_then(timing),
                "details":fields.as_ref().map(crate::journal_report::delivery::report),
                "shortSummary":fields.as_ref().and_then(crate::journal_report::delivery::compact)}))
        },
        Some("entry")=> {let mut v=c["entry"].clone();v["time"]=json!(1);Ok(json!({"valid":filtered_entry(&v).is_some()}))},
        Some("journal")=>Ok(json!({"entries":journal(&c["entries"])})),
        Some("append")=> {let mut a=c["entries"].as_array().cloned().unwrap_or_default();a.push(c["entry"].clone());Ok(json!({"entries":journal(&json!(a))}))},
        Some("decodeState")=> {
            let v=&c["value"];let s=state(&json!({"outcome":v["lastOutcome"],"reason":v["lastReason"]}));
            Ok(json!({"last":s,"pendingFailures":{}}))
        },
        Some("encodeState")=> {
            Ok(json!({"value":state(&c["last"]).map(|s|json!({"lastOutcome":s["outcome"],"lastReason":s["reason"]})).unwrap_or(json!({}))}))
        },
        Some("counters")=> {
            let reason=c["reason"].as_str().map(str::to_ascii_lowercase);
            let values=r::Counters::binding(counts(&c["pending"]),reason.as_deref(),counts(&c["restore"]));
            Ok(json!({"pending":values.into_iter().map(|(k,v)|(k.to_ascii_uppercase(),v)).collect::<BTreeMap<_,_>>()}))
        },
        Some("wire")=>Ok(json!({"schemaVersion":1,"failures":counts(&c["failures"])})),
        Some("fallback")=>Ok(json!({"reason":if c["reason"]=="OFFLINE" {"CONFIRMED_OFFLINE"} else {c["reason"].as_str().filter(|s|REASONS.contains(s)).ok_or(Error::InvalidRequest)?}})),
        _=>Err(Error::InvalidRequest),
    }
}
