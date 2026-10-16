use crate::{update_install::Candidate, Services};
use tauri::{AppHandle, Manager};

/// Tauri owns package verification/extraction/replacement on both platforms.
/// The host only drains its existing recording/account lifetime before install.
pub async fn apply(app: &AppHandle, candidate: Candidate) -> (Option<Candidate>, Result<(), &'static str>) {
    if candidate.model_free && !crate::update_install::retain_model(app).await {
        return (Some(candidate), Err("updates_model_required"));
    }
    let services = app.state::<Services>().inner().clone();
    if let Err(error) = services.stop_for_maintenance().await {
        let _ = services.resume_after_maintenance(app).await;
        return (Some(candidate), Err(error));
    }
    let installation = tauri::async_runtime::spawn_blocking(move || {
        let result = candidate.update.install(&candidate.bytes).map_err(|_| "updates_install_failed");
        (candidate, result)
    }).await;
    match installation {
        Ok((candidate, result)) => {
            if result.is_err() { let _ = services.resume_after_maintenance(app).await; }
            (Some(candidate), result)
        }
        Err(_) => {
            let _ = services.resume_after_maintenance(app).await;
            (None, Err("updates_recovery_required"))
        }
    }
}
