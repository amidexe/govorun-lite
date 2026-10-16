//! Package choice only; Tauri still downloads, verifies and installs both forms.
use serde_json::Value;

pub struct Asset { pub url: url::Url, pub signature: String, pub model_free: bool }

pub fn select(raw: &Value, platform: &str, revision: &str, retained: bool) -> Result<Option<Asset>, &'static str> {
    if raw.get("modelRevision").is_none() { return Ok(None); }
    let force_full = raw.get("forceFull").map(|value| value.as_bool().ok_or("updates_invalid_feed")).transpose()?.unwrap_or(false);
    let model_free = !force_full && retained && raw.get("modelRevision").and_then(Value::as_str) == Some(revision);
    let group = if model_free { "modelFreePlatforms" } else { "fullPlatforms" };
    let item = raw.get(group).and_then(|group| group.get(platform)).ok_or("updates_invalid_feed")?;
    let url = item.get("url").and_then(Value::as_str).ok_or("updates_invalid_feed")?;
    let signature = item.get("signature").and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 4096).ok_or("updates_invalid_feed")?;
    Ok(Some(Asset { url: govorun_core::update_policy::official_url(url)?, signature: signature.into(), model_free }))
}
