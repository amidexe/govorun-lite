//! Optional notifications: local consent, revision fences and anonymous registration.
//! Account scope never enters provider or registry requests. SDK/storage are ports.
use crate::{Error, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
const WEEK: i64 = 7 * 86400;
fn number(v:&Value, k:&str)->i64 {v[k].as_i64().unwrap_or(0)}
fn text<'a>(v:&'a Value,k:&str)->&'a str {v[k].as_str().unwrap_or("")}
fn flag(v:&Value,k:&str)->bool {v[k]==true}
fn enabled(s:&Value)->bool {flag(s,"updates") || flag(s,"offers")}
fn deliver(s:&Value,scope:&str,permission:bool)->bool {
    !scope.is_empty() && text(s,"accountScope")==scope && permission && enabled(s)
}
fn accepts(s:&Value,category:&str)->bool {matches!(category,"updates"|"offers") && flag(s,category)}
fn invite(s:&Value,scope:&str,supported:bool,now:i64)->bool {
    let start=number(s,"firstSeenAt").max(number(s,"signedInAt"));
    supported && !scope.is_empty() && text(s,"accountScope")==scope && !flag(s,"prompted") && !enabled(s)
        && number(s,"firstSeenAt")>0 && number(s,"signedInAt")>0 && now>=start && now-start>=WEEK
}
fn consent_matches(s:&Value,request:&Value,scope:&str)->bool {
    !scope.is_empty() && text(s,"accountScope")==scope && text(request,"accountScope")==scope
        && number(request,"revision")==number(s,"revision")
}
#[path="notice.rs"] mod notice;
pub(crate) fn evaluate(c:Value)->Result<Value> {
    let mut s=c["state"].clone(); let scope=text(&c,"scope"); let now=number(&c,"now");
    let permission=flag(&c,"permission"); let revision=number(&s,"revision");
    let pending=revision>number(&s,"acknowledged") || flag(&s,"sdkRemovalPending");
    match text(&c,"kind") {
        "scope" => Ok(json!({"scope":if flag(&c,"available") && flag(&c,"signedIn") && !flag(&c,"storageProblem")
            && !text(&c,"accountId").trim().is_empty() && !text(&c,"origin").trim().is_empty() {
                format!("{:x}",Sha256::digest(format!("{}\n{}",text(&c,"origin"),text(&c,"accountId"))))
            } else {String::new()}})),
        "accepts" => Ok(json!({"allowed":accepts(&s,text(&c,"category"))})),
        "enabled" => Ok(json!({"allowed":enabled(&s)})),
        "invite" => Ok(json!({"allowed":invite(&s,scope,flag(&c,"supported"),now)})),
        "deliver" => Ok(json!({"allowed":deliver(&s,scope,permission)})),
        "canAccept" => Ok(json!({"allowed":consent_matches(&s,&c["request"],scope)})),
        "firstSeen" => {if number(&s,"firstSeenAt")<=0 && now>0 {s["firstSeenAt"]=json!(now);} Ok(s)},
        "account" => {
            let changed=text(&s,"accountScope")!=scope;
            if !changed && (!enabled(&s) || (!scope.is_empty() && permission)) {
                if !scope.is_empty() && number(&s,"signedInAt")==0 && now>0 {s["signedInAt"]=json!(now);}
            } else {
                s["accountScope"]=json!(scope);
                s["signedInAt"]=json!(if scope.is_empty() {0} else if changed || number(&s,"signedInAt")==0 {now} else {number(&s,"signedInAt")});
                s["updates"]=json!(false); s["offers"]=json!(false); s["prompted"]=json!(false);
                s["revision"]=json!(revision+1); s["sdkRemovalPending"]=json!(flag(&s,"sdkRemovalPending") || !text(&s,"id").is_empty());
                s["notices"]=json!({});
            } Ok(s)
        },
        "later" => {s["prompted"]=json!(true); s["revision"]=json!(revision+1); Ok(s)},
        "accept" => {
            if !permission || !flag(&c,"storageAvailable") || !flag(&c,"configured") || !flag(&c,"supported") || !consent_matches(&s,&c["request"],scope) {return Ok(Value::Null);}
            s["updates"]=json!(true); s["offers"]=json!(true); s["prompted"]=json!(true);
            s["revision"]=json!(revision+1); s["sdkRemovalPending"]=json!(false); Ok(s)
        },
        "receive" => notice::receive(s,c["notice"].clone(),permission,now),
        "parse" => Ok(notice::parse(&c["data"],text(&c,"package"),number(&c,"version"),now).unwrap_or(Value::Null)),
        "syncPlan" => Ok(json!({"schedule":!text(&s,"accountScope").is_empty() && flag(&c,"storageAvailable") && flag(&c,"configured") && !text(&s,"id").is_empty()
            && (enabled(&s) || pending),"urgent":pending,"reconcile":!text(&s,"accountScope").is_empty() && flag(&c,"storageAvailable") && flag(&c,"configured") && !text(&s,"id").is_empty(),"token":enabled(&s),
            "remove":flag(&s,"sdkRemovalPending") && !enabled(&s),"sync":revision>number(&s,"acknowledged"),
            "missingToken":enabled(&s) && text(&s,"token").is_empty(),
            "cancel":!enabled(&s) && revision==number(&s,"acknowledged") && !flag(&s,"sdkRemovalPending")})),
        "token" => {
            if deliver(&s,scope,permission) && revision==number(&c,"expectedRevision") &&
                (text(&c,"token")!=text(&s,"token") || number(&s,"versionCode")!=number(&c,"version") || now-number(&s,"syncedAt")>=86400) {
                    s["token"]=c["token"].clone(); s["versionCode"]=c["version"].clone(); s["revision"]=json!(revision+1);
                } Ok(s)
        },
        "removed" => {if !enabled(&s) && revision==number(&c,"expectedRevision") {
            s["token"]=json!("");s["sdkRemovalPending"]=json!(false);} Ok(s)},
        "acknowledged" => {if revision==number(&c,"expectedRevision") {s["acknowledged"]=json!(revision);s["syncedAt"]=json!(now);} Ok(s)},
        "ui" => {
            let mut notices:Vec<_>=s["notices"].as_object().into_iter().flat_map(|v|v.values())
                .filter(|n|number(n,"expiresAt")>now).cloned().collect();
            notices.sort_by_key(|n|std::cmp::Reverse(number(n,"publishedAt")));
            Ok(json!({"invite":invite(&s,scope,flag(&c,"supported"),now),"pending":!scope.is_empty() && pending,"notices":notices,
                "updates":deliver(&s,scope,permission) && flag(&s,"updates"),"offers":deliver(&s,scope,permission) && flag(&s,"offers")}))
        },
        "request" => {
            let mut body=json!({"id":s["id"],"revision":revision});
            if enabled(&s) {body["provider"]=json!("fcm");body["package"]=c["package"].clone();body["versionCode"]=s["versionCode"].clone();
                body["token"]=s["token"].clone();body["consentVersion"]=json!(1);body["categories"]=json!({"updates":flag(&s,"updates"),"offers":flag(&s,"offers")});}
            Ok(json!({"method":if enabled(&s) {"PUT"} else {"DELETE"},"path":"/v1/notifications/registration","body":body}))
        },
        _=>Err(Error::InvalidRequest),
    }
}
