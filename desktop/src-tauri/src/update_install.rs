use crate::update_cancel::Cancellation;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::Update;

pub struct Candidate { pub update: Update, pub bytes: Vec<u8>, pub model_free: bool }

pub async fn retain_model(app: &AppHandle) -> bool {
    let paths = (|| {
        crate::model_store::application_paths(&app.path().resource_dir().ok()?,
            &app.path().local_data_dir().ok()?, &app.config().identifier).ok()
    })();
    let Some((bundled, root)) = paths else { return false; };
    tauri::async_runtime::spawn_blocking(move || crate::model_store::PRODUCTION.retain(&bundled, &root).is_ok())
        .await.unwrap_or(false)
}

pub fn public_key(identifier: &str) -> Result<String, &'static str> {
    let keys: serde_json::Value = serde_json::from_str(include_str!("../update-public-keys.json"))
        .map_err(|_| "updates_unavailable")?;
    let profile = if identifier == "com.govorun.app.testbackend" { "test" } else { "ordinary" };
    keys[profile].as_str().map(str::to_string).ok_or("updates_unavailable")
}


pub async fn prepare(app: &AppHandle, mut update: Update, cancel: Arc<Cancellation>, progress: impl Fn(u64, u64)) -> Result<Candidate, &'static str> {
    let mut model_free = update.raw_json.get("modelFreeDefault").and_then(serde_json::Value::as_bool).unwrap_or(false);
    if update.raw_json.get("modelRevision").is_some() {
        let retained = if update.raw_json.get("forceFull").and_then(serde_json::Value::as_bool) == Some(true) {
            false
        } else { retain_model(app).await };
        if cancel.requested() { return Err("updates_cancelled"); }
        let platform = format!("{}-{}", update.target, std::env::consts::ARCH);
        if let Some(asset) = crate::update_asset::select(&update.raw_json, &platform,
                &crate::model_store::PRODUCTION.identity(), retained)? {
            update.download_url = asset.url; update.signature = asset.signature; model_free = asset.model_free;
        }
    }
    let mut downloaded = 0u64;
    let download = update.download(|chunk, total| {
        downloaded = downloaded.saturating_add(chunk as u64);
        if let Some(total) = total.filter(|total| *total > 0) { progress(downloaded, total); }
    }, || {});
    let bytes = tokio::select! {
        biased;
        _ = cancel.wait() => return Err("updates_cancelled"),
        result = download => result.map_err(|_| "updates_download_failed")?,
    };
    Ok(Candidate { update, bytes, model_free })
}
