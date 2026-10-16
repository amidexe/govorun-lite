//! Accounts pass requests and reserve-response admission, shared with JNI.
use crate::{AccountRequest, Method, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Command {
    Key,
    Issue { id: String, key_id: String, messages: Vec<String>, receipts: Vec<String> },
    Deposit { id: String, scope_id: String, receipts: Vec<String> },
    DepositResponse { body: Value, count: usize },
    RejectedDeposit { status: u16, body: Value },
}
pub(crate) fn issue_request(id: String, key_id: String, messages: Value, receipts: Vec<Value>) -> Result<AccountRequest> {
    let mut body=json!({"keyId":key_id,"blindedMessages":messages});
    if !receipts.is_empty() { body["replacementReceipts"]=json!(receipts); }
    AccountRequest::new(Method::Post,"/v1/style-passes/issue",Some(body))?.with_idempotency_key(id)
}
pub(crate) fn key() -> Result<AccountRequest> { AccountRequest::new(Method::Get,"/v1/style-passes/key",None) }
pub(crate) fn deposit_request(id: String, scope_id: String, receipts: Vec<String>) -> Result<AccountRequest> {
    AccountRequest::new(Method::Post,"/v1/style-passes/deposit",
        Some(json!({"scopeId":scope_id,"receipts":receipts})))?.with_idempotency_key(id)
}
pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let request=match command {
        Command::Key => key()?,
        Command::Issue { id,key_id,messages,receipts } => issue_request(id,key_id,json!(messages),receipts.into_iter().map(Value::String).collect())?,
        Command::Deposit { id,scope_id,receipts } => deposit_request(id,scope_id,receipts)?,
        Command::DepositResponse { body,count } => {
            if body["returnedCount"].as_u64()!=Some(count as u64) || !body["styleAccess"].is_object() { return Err(Error::InvalidResponse); }
            return Ok(body["styleAccess"].clone());
        }
        Command::RejectedDeposit { status,body } => return Ok(json!({"rejected":crate::style_issue::failure(
            Some(status),crate::style_issue::FailureBody::from_value(&body))?.discard_replacement_receipts})),
    };
    let headers=request.idempotency_key().map(|id|json!({"Idempotency-Key":id})).unwrap_or_else(||json!({}));
    Ok(json!({"method":request.method(),"path":request.path(),"body":request.body(),"headers":headers}))
}
