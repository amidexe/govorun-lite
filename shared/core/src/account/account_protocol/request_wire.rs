//! Named request view. Credentials, epoch admission and HTTP stay in the
//! persistent account owner; this binding cannot select arbitrary paths.
use crate::{Error, Result, AccountRequest, Method};
use serde_json::{json, Value};
pub(crate) fn evaluate(mut command: Value) -> Result<Value> {
    command["sessionGeneration"]=json!(0);
    let operation=command["operation"].as_str().ok_or(Error::InvalidRequest)?.to_owned();
    let mut wire=if operation.starts_with("sync") {super::sync::request(command)?}
        else if operation=="styleUsageWrite" {
            let body=command["report"].clone();
            let report:crate::usage_ledger::Report=serde_json::from_value(body.clone()).map_err(|_|Error::InvalidRequest)?;
            crate::usage_ledger::validation::report(&report)?;
            usage_request(body)?
        } else {super::account::request(command)?};
    let mut headers=json!({});
    if let Some(etag)=wire["etag"].as_str() {
        headers[if wire["method"]=="GET" {"If-None-Match"} else {"If-Match"}]=json!(etag);
    }
    wire["acceptedStatuses"]=if operation=="syncContentRead" {json!([200,304])} else {json!([])};
    wire["headers"]=headers;
    wire.as_object_mut().ok_or(Error::InvalidRequest)?.remove("sessionGeneration");
    Ok(wire)
}
pub(crate) fn usage_request(body:Value) -> Result<Value> {
    let request=AccountRequest::new(Method::Put,"/v1/style-usage",Some(body))?;
    Ok(json!({"method":request.method(),"path":request.path(),"body":request.body()}))
}
