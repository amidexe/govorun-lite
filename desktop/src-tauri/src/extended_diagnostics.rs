use govorun_core::operational_trace::OperationalTrace;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, io::Write, path::PathBuf, sync::Mutex, time::{Instant, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Emitter, Manager};

const MAXIMUM: usize = 200;
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored { enabled: bool, events: Vec<(u64, OperationalTrace)> }
pub struct ExtendedDiagnostics { path: Option<PathBuf>, stored: Mutex<Stored> }

impl ExtendedDiagnostics {
    pub fn load(app: &AppHandle) -> Self {
        let path = app.path().app_config_dir().ok().map(|p| p.join("extended-diagnostics.json"));
        Self::at_path(path)
    }
    fn at_path(path: Option<PathBuf>) -> Self {
        let mut stored = path.as_ref().and_then(|p| fs::read(p).ok()).filter(|b| b.len() <= 131_072)
            .and_then(|b| serde_json::from_slice::<Stored>(&b).ok()).filter(|s| s.events.len() <= MAXIMUM).unwrap_or_default();
        stored.events.retain(|(_, event)| event.row(0).is_some());
        Self { path, stored: Mutex::new(stored) }
    }
    fn save(&self, value: &Stored) -> Result<(), &'static str> {
        let path = self.path.as_ref().ok_or("diagnostics_storage_unavailable")?;
        fs::create_dir_all(path.parent().ok_or("diagnostics_storage_unavailable")?).map_err(|_| "diagnostics_save_failed")?;
        let pending = path.with_extension("pending");
        fs::write(&pending, serde_json::to_vec(value).map_err(|_| "diagnostics_save_failed")?).map_err(|_| "diagnostics_save_failed")?;
        fs::rename(&pending, path).map_err(|_| "diagnostics_save_failed")
    }
    pub fn record(&self, app: &AppHandle, operation: &str, error: Option<&str>, duration: u64) {
        let Some(now) = SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|n| i64::try_from(n.as_millis()).ok()) else { return; };
        if !self.record_values(operation, error, duration, now) { return; }
        let _ = app.emit("govorun:event", json!({"event":"diagnostics.changed", "data":null}));
    }
    fn record_values(&self, operation: &str, error: Option<&str>, duration: u64, now: i64) -> bool {
        let Some(entry) = OperationalTrace::new(operation, error, now, duration) else { return false; };
        let Ok(mut stored) = self.stored.lock() else { return false; };
        if !stored.enabled { return false; }
        let ordinal = stored.events.last().map(|(n,_)| n.saturating_add(1)).unwrap_or(0);
        stored.events.push((ordinal, entry));
        if stored.events.len() > MAXIMUM { stored.events.remove(0); }
        let _ = self.save(&stored);
        true
    }
    pub fn record_system(&self, app: &AppHandle, start_failed: bool, code: Option<i32>) {
        let Some(now) = SystemTime::now().duration_since(UNIX_EPOCH).ok().and_then(|time| i64::try_from(time.as_millis()).ok()) else { return; };
        let Ok(mut stored) = self.stored.lock() else { return; };
        if !stored.enabled { return; }
        let entry = if start_failed { crate::system_diagnostics::worker_start_failed(now, code) }
            else { crate::system_diagnostics::worker_exit(now, code) };
        let ordinal = stored.events.last().map(|(n,_)| n.saturating_add(1)).unwrap_or(0);
        stored.events.push((ordinal, entry));
        if stored.events.len() > MAXIMUM { stored.events.remove(0); }
        let _ = self.save(&stored);
        drop(stored);
        let _ = app.emit("govorun:event", json!({"event":"diagnostics.changed", "data":null}));
    }
    pub fn snapshot(&self, mut native: Value) -> Result<Value, &'static str> {
        let stored = self.stored.lock().map_err(|_| "diagnostics_unavailable")?;
        let native_available = native.get("events").is_some_and(Value::is_array);
        let mut rows: Vec<Value> = native.get("events").and_then(Value::as_array).cloned().unwrap_or_default();
        rows.truncate(MAXIMUM);
        let mut extended_rows: Vec<Value> = stored.events.iter().filter_map(|(n, e)| usize::try_from(*n).ok().and_then(|n| e.row(n))).collect();
        if let Some(row) = crate::installation_diagnostics::current() { extended_rows.push(row); }
        extended_rows.sort_by(|a,b| b["date"].as_str().cmp(&a["date"].as_str())); extended_rows.truncate(MAXIMUM);
        native = json!({"maximumEvents": MAXIMUM, "events": rows, "extendedEvents":extended_rows,
            "extendedEnabled":stored.enabled, "nativeAvailable":native_available});
        Ok(native)
    }
    pub fn configure(&self, enabled: bool) -> Result<(), &'static str> {
        let mut stored = self.stored.lock().map_err(|_| "diagnostics_unavailable")?;
        let next = Stored { enabled, events: stored.events.clone() };
        self.save(&next)?; *stored = next; Ok(())
    }
    pub fn clear(&self) -> Result<(), &'static str> {
        let mut stored = self.stored.lock().map_err(|_| "diagnostics_unavailable")?;
        let next = Stored { enabled: stored.enabled, events: vec![] };
        self.save(&next)?; *stored = next; Ok(())
    }
}

pub async fn command(app: &AppHandle, services: &crate::service::Services, method: &str, params: Value) -> Result<Value, &'static str> {
    let trace = app.state::<ExtendedDiagnostics>();
    if method == "diagnostics.snapshot" {
        if !params.as_object().is_some_and(|p|p.is_empty()) { return Err("invalid_command"); }
        return trace.snapshot(services.invoke(method, json!({})).await.unwrap_or_else(|_| json!({})));
    }
    match params.get("action").and_then(Value::as_str) {
        Some("extended") if params.as_object().is_some_and(|p| p.len() == 2) => {
            trace.configure(params["enabled"].as_bool().ok_or("invalid_command")?)?;
            let _ = app.emit("govorun:event", json!({"event":"diagnostics.changed", "data":null}));
            trace.snapshot(services.invoke("diagnostics.snapshot", json!({})).await.unwrap_or_else(|_| json!({})))
        }
        Some("clear") if params["confirmed"] == true && params.as_object().is_some_and(|p| p.len() == 2) => {
            let native = services.invoke(method, json!({"action":"clear", "confirmed":true})).await?;
            crate::installation_diagnostics::clear()?;
            trace.clear()?; trace.snapshot(native)
        }
        Some("copy") if params.as_object().is_some_and(|p| p.len() == 1) => {
            services.invoke(method, json!({"action":"copy"})).await
        }
        Some(action @ ("copyReport" | "export")) if params.as_object().is_some_and(|p| p.len() == 1) => {
            let native = services.invoke("diagnostics.snapshot", json!({})).await;
            let snapshot = trace.snapshot(native.clone().unwrap_or_else(|_| json!({})))?;
            let mut text = format!("Говорун {} · {}\n", app.package_info().version, std::env::consts::OS);
            if native.is_err() { text.push_str("Основной журнал недоступен: обработчик приложения не отвечает.\n"); }
            for (key, heading) in [("events", "События приложения"), ("extendedEvents", "Расширенная диагностика")] {
                text.push_str(&format!("\n{heading}\n"));
                for row in snapshot[key].as_array().ok_or("diagnostics_unavailable")? {
                    text.push_str(&format!("{} · {} · {}\n", row["date"].as_str().unwrap_or(""), row["category"].as_str().unwrap_or(""), row["summary"].as_str().unwrap_or("")));
                    for field in ["details", "timing"] { if let Some(value) = row[field].as_str() { text.push_str(value); text.push('\n'); } }
                }
            }
            if action == "export" {
                let directory = app.path().download_dir().map_err(|_| "diagnostics_save_failed")?;
                save_report(&directory, &text)?;
                Ok(Value::Null)
            }
            else { services.copy_diagnostic_report(text).await }
        }
        _ => Err("invalid_command"),
    }
}

fn save_report(directory: &std::path::Path, text: &str) -> Result<(), &'static str> {
    if text.len() > 131_072 { return Err("diagnostics_save_failed"); }
    fs::create_dir_all(directory).map_err(|_| "diagnostics_save_failed")?;
    let time = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "diagnostics_save_failed")?.as_millis();
    // A new file on each request; never overwrite an existing user file.
    let path = directory.join(format!("Govorun-diagnostics-{time}.txt"));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_| "diagnostics_save_failed")?;
    if file.write_all(text.as_bytes()).is_err() {
        drop(file);
        let _ = fs::remove_file(path);
        return Err("diagnostics_save_failed");
    }
    Ok(())
}

pub fn finished(app: &AppHandle, operation: &str, start: Instant, error: Option<&str>) {
    app.state::<ExtendedDiagnostics>().record(app, operation, error, start.elapsed().as_millis().min(600_000) as u64);
}
