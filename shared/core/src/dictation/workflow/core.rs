//! In-process shared decisions. There is no native business RPC here.
use super::Failure;
use serde_json::{json, Value};

pub(crate) fn evaluate(command: Value) -> Result<Value, Failure> {
    let response = govorun_core::evaluate_business_command(
        &json!({"operation":"dictationPreflight","command":command}).to_string(),
    );
    let response: Value = serde_json::from_str(&response)
        .map_err(|_| Failure::Effect("dictation_invalid_core_result"))?;
    response
        .get("result")
        .cloned()
        .ok_or(Failure::Effect("dictation_invalid_core_result"))
}
