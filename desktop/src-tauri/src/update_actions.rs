use super::{Snapshot, Transfer, Updates};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Emitter, Manager};

pub(super) struct Active<'a>(&'a AtomicBool);
impl Drop for Active<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }

impl Updates {
    pub(super) fn enter(&self) -> Result<Active<'_>, &'static str> {
        self.operation.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Active(&self.operation)).map_err(|_| "updates_busy")
    }

    fn changed(&self, app: &AppHandle) -> Result<Snapshot, &'static str> {
        let snapshot = {
            let value = self.value.lock().map_err(|_| "updates_unavailable")?;
            Self::snapshot(&value)
        };
        crate::attention::event(app, "updates.changed", &serde_json::to_value(&snapshot).unwrap_or_default());
        let _ = app.emit("govorun:event", serde_json::json!({"event":"updates.changed", "data":snapshot}));
        Ok(snapshot)
    }

    pub(super) fn cancel_download(&self, app: &AppHandle) -> Result<Snapshot, &'static str> {
        {
            let value = self.value.lock().map_err(|_| "updates_unavailable")?;
            if let Some(cancel) = &value.cancel { cancel.request(); }
        }
        self.changed(app)
    }

    pub(super) async fn start_download(&self, app: &AppHandle, expected_version: Option<&str>) -> Result<Snapshot, &'static str> {
        let account = app.state::<crate::service::Services>().account_session().await;
        let mut boundary = account.as_ref().map(|owner| owner.subscribe_boundary());
        let active = self.enter()?;
        let update = {
            let value = self.value.lock().map_err(|_| "updates_unavailable")?;
            let release = value.release.as_ref().ok_or("updates_stale_release")?;
            if expected_version != Some(release.version.as_str()) { return Err("updates_stale_release"); }
            // Download exactly the Update returned by the previous Tauri check.
            // No second manifest request can replace the selected release.
            value.available.clone().ok_or("updates_stale_release")?
        };
        let cancel = Arc::new(crate::update_cancel::Cancellation::default());
        {
            let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
            value.status = "downloading"; value.error = None; value.transfer = None;
            value.candidate = None; value.cancel = Some(cancel.clone());
        }
        self.changed(app)?;
        let prepare = crate::update_install::prepare(app, update, cancel.clone(), |downloaded, total| {
            let changed = if let Ok(mut value) = self.value.lock() {
                let previous = value.transfer.as_ref().map(|p| p.downloaded.saturating_mul(100) / p.total.max(1));
                value.transfer = Some(Transfer { downloaded, total });
                previous != Some(downloaded.saturating_mul(100) / total.max(1)) || downloaded == total
            } else { false };
            if changed { let _ = self.changed(app); }
        });
        tokio::pin!(prepare);
        let result = tokio::select! {
            biased;
            _ = async {
                match &mut boundary {
                    Some(changes) => { let _ = changes.changed().await; },
                    None => std::future::pending().await,
                }
            } => { cancel.request(); prepare.await },
            result = &mut prepare => result,
        };
        let ready = result.is_ok();
        {
            let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
            value.cancel = None;
            match result {
                Ok(candidate) => { value.candidate = Some(candidate); value.status = "ready"; }
                Err(error) => { value.status = "available"; value.transfer = None; value.error = Some(error); }
            }
        }
        self.changed(app)?;
        drop(active);
        // The explicit Update action may install immediately when native confirms
        // it is idle. A busy dictation leaves the verified package ready for later.
        if ready { self.install_candidate(app, expected_version).await } else { self.changed(app) }
    }

    pub(super) async fn install_candidate(&self, app: &AppHandle, expected_version: Option<&str>) -> Result<Snapshot, &'static str> {
        let _active = self.enter()?;
        let candidate = {
            let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
            let release = value.release.as_ref().ok_or("updates_stale_release")?;
            if expected_version != Some(release.version.as_str()) { return Err("updates_stale_release"); }
            let candidate = value.candidate.take().ok_or("updates_no_package")?;
            value.status = "installing"; value.error = None;
            candidate
        };
        self.changed(app)?;
        let (candidate, result) = crate::update_apply::apply(app, candidate).await;
        match result {
            Ok(()) => app.restart(),
            Err(error) => {
                let mut value = self.value.lock().map_err(|_| "updates_unavailable")?;
                let candidate = if error == "updates_model_required" { None } else { candidate };
                value.status = if candidate.is_some() { "ready" } else { "available" };
                value.candidate = candidate; value.error = Some(error);
            }
        }
        self.changed(app)
    }
}
