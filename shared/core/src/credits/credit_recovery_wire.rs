use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
enum Response {
    Ready {},
    Consumed {},
    Pending { #[serde(rename = "retryAfterMs")] delay: u64 },
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Rejection { error: String }

#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Decision {
    pub valid_response: bool,
    pub disposition: &'static str,
    pub retry_after_millis: Option<u64>,
}

pub(crate) fn decide(status: Option<u16>, body: &str) -> Decision {
    let waiting = |delay, valid_response| Decision { valid_response, disposition: "wait", retry_after_millis: Some(delay) };
    let terminal = |disposition| Decision { valid_response: true, disposition, retry_after_millis: None };
    if status == Some(401) && serde_json::from_str::<Rejection>(body)
        .is_ok_and(|r| r.error == "invalid_style_pass") {
        return terminal("discard");
    }
    if status != Some(200) { return waiting(50_000, false); }
    match serde_json::from_str::<Response>(body) {
        Ok(Response::Ready {}) => terminal("restore"),
        Ok(Response::Consumed {}) => terminal("discard"),
        Ok(Response::Pending { delay }) if (1..=50_000).contains(&delay) => waiting(delay.max(250), true),
        _ => waiting(50_000, false),
    }
}

pub(crate) fn bounded_delay(milliseconds: Option<i64>) -> u64 {
    milliseconds.unwrap_or(250).clamp(250, 50_000) as u64
}

pub(crate) fn request(passes: Vec<String>) -> crate::Result<serde_json::Value> {
    if !(1..=crate::style_inventory::BATCH).contains(&passes.len()) || passes.iter().any(|pass| pass.trim().is_empty() || pass.len() > 16_384)
        || passes.iter().collect::<std::collections::BTreeSet<_>>().len() != passes.len() {
        return Err(crate::Error::InvalidRequest);
    }
    Ok(serde_json::json!({"stylePasses":passes}))
}
