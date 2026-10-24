//! Physical journal and explicit clipboard effects; filtering/append policy is Core.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
use govorun_core::journal_report::{
    input::{self, Input},
    Journal as Report,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
pub type Changed = Arc<dyn Fn() + Send + Sync>;
pub struct Journal {
    serial: Mutex<()>,
    changed: OnceLock<Changed>,
    #[cfg(target_os = "macos")]
    store: super::database_defaults::Journal<Arc<super::macos_defaults::Defaults>>,
    #[cfg(target_os = "windows")]
    store: Arc<super::database_files::Documents>,
}
impl Journal {
    /// The journal is a row of the documents base on both desktops since 2.1.112.
    #[cfg(target_os = "macos")]
    pub fn new(store: Arc<super::database_defaults::Documents<Arc<super::macos_defaults::Defaults>>>) -> Self {
        Self {
            store: super::database_defaults::Journal(store),
            serial: Mutex::new(()),
            changed: OnceLock::new(),
        }
    }
    #[cfg(target_os = "windows")]
    pub fn new(store: Arc<super::database_files::Documents>) -> Self {
        Self {
            store,
            serial: Mutex::new(()),
            changed: OnceLock::new(),
        }
    }
    pub fn bind(&self, changed: Changed) -> Result<(), &'static str> {
        self.changed
            .set(changed)
            .map_err(|_| "diagnostics_already_composed")
    }
    fn report(&self) -> Result<Report, &'static str> {
        #[cfg(target_os = "macos")]
        {
            Ok(Report::Macos(macos::read(&self.store)?))
        }
        #[cfg(target_os = "windows")]
        {
            let mut events = windows::read(&self.store.scope(windows::OWNER))?;
            events.reverse();
            Ok(Report::Windows(events))
        }
    }
    fn facts(&self) -> Result<Value, &'static str> {
        match self.report()? {
            Report::Macos(entries) => Ok(json!({"platform":"macos","entries":entries})),
            Report::Windows(entries) => Ok(json!({"platform":"windows","entries":entries})),
        }
    }
    fn append(&self, value: &Value) -> Result<(), &'static str> {
        let input = input::parse(value).map_err(|_| "invalid_parameters")?;
        let now = chrono::Utc::now();
        #[cfg(target_os = "macos")]
        {
            use govorun_core::journal_report::MacEvent;
            let (message, category, level, timing, deduplicate) = match input {
                Input::CaptureFailure(code) => {
                    return macos::append(&self.store, MacEvent {
                        id: uuid::Uuid::new_v4().to_string().to_uppercase(),
                        date: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        level: "error".into(), category: "Запись".into(),
                        message: govorun_core::journal_report::capture_failure::stored(&code).ok_or("invalid_parameters")?,
                        timing: None,
                    }, false);
                }
                Input::InsertionFailure(code) => {
                    let message = govorun_core::journal_report::insertion::stored(code).ok_or("invalid_parameters")?;
                    return macos::append(&self.store, MacEvent {
                        id: uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase(),
                        date: now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true), level: "error".into(),
                        category: govorun_core::journal_report::insertion::CATEGORY.into(), message, timing: None,
                    }, false);
                }
                Input::CaptureTiming(_) => return Err("invalid_parameters"),
                Input::Timing(timing) => ("", "Стили", "info", Some(timing), false),
                Input::RecognitionCompleted => ("Распознавание завершено.", "Распознавание", "info", None, false),
                Input::Failure {
                    name: "self_host_request_failed",
                    status,
                    ..
                } => return macos::append(&self.store, MacEvent {
                    id: uuid::Uuid::new_v4().to_string().to_uppercase(),
                    date: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    level: "error".into(), category: "Стили".into(),
                    message: input::self_host_failure_message(status), timing: None,
                }, false),
                Input::Failure {
                    name: "recording_feedback_failed",
                    ..
                } => (
                    "Не удалось выполнить операцию.",
                    "Запись",
                    "error",
                    None,
                    true,
                ),
                // The move into the base is a fact, not a failure.
                Input::Failure { name: "local_data_moved", reason, .. } => return macos::append(&self.store, MacEvent {
                    id: uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase(),
                    date: now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
                    level: "info".into(), category: "Приложение".into(),
                    message: input::failure_message("Перенос локальных данных", None, reason.as_deref()), timing: None,
                }, true),
                Input::Failure { name, status, reason } => {
                    let (title, category) = match name {
                        "content_sync_failed" => ("Не удалось синхронизировать настройки", "Аккаунт"),
                        "preferences_sync_failed" => ("Не удалось синхронизировать параметры", "Аккаунт"),
                        "account_sync_driver_failed" => ("Не удалось продолжить синхронизацию", "Аккаунт"),
                        "settings_load_failed" => ("Не удалось загрузить настройки", "Приложение"),
                        "dictionary_load_failed" => ("Не удалось загрузить словарь", "Приложение"),
                        "statistics_journal_unavailable" => ("Не удалось прочитать сохранённую статистику", "Статистика"),
                        "sync_state_load_failed" => ("Не удалось проверить синхронизацию", "Аккаунт"),
                        "billing_action_failed" => ("Не удалось выполнить действие", "Подписка"),
                        _ => ("Не удалось синхронизировать статистику", "Статистика"),
                    };
                    return macos::append(&self.store, MacEvent {
                        id: uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase(),
                        date: now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
                        level: "warning".into(), category: category.into(),
                        message: input::failure_message(title, status, reason.as_deref()), timing: None,
                    }, true);
                }
            };
            macos::append(
                &self.store,
                MacEvent {
                    id: uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase(),
                    date: now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
                    level: level.into(),
                    category: category.into(),
                    message: message.into(),
                    timing,
                },
                deduplicate,
            )
        }
        #[cfg(target_os = "windows")]
        {
            use chrono::Timelike;
            use govorun_core::journal_report::WindowsEvent;
            let (name, detail) = match input {
                Input::Timing(timing) => ("stylization_timing", timing),
                Input::RecognitionCompleted => ("asr_completed", "".into()),
                Input::InsertionFailure(code) => ("insertion_failed", code.into()),
                Input::CaptureTiming(detail) => ("audio_capture_ready", detail),
                Input::CaptureFailure(code) => ("audio_capture_failed", code.into()),
                Input::Failure { name, status, reason } => (
                    name,
                    status
                        .map(|code| format!("http_{code}"))
                        .or(reason)
                        .unwrap_or_default(),
                ),
            };
            windows::append(
                &self.store.scope(windows::OWNER),
                WindowsEvent {
                    date: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    identity_date: format!(
                        "{}.{:07}+00:00",
                        now.format("%Y-%m-%dT%H:%M:%S"),
                        now.nanosecond() / 100
                    ),
                    name: name.into(),
                    detail,
                    timing: None,
                },
            )
        }
    }
    pub fn invoke(&self, method: &str, params: &Value) -> Result<Value, &'static str> {
        let _guard = self.serial.lock().map_err(|_| "storage_unavailable")?;
        if method == "diagnostics.localFacts" {
            if !params.as_object().is_some_and(|fields| fields.is_empty()) {
                return Err("invalid_parameters");
            }
            return self.facts();
        }
        if method == "diagnostics.localEffect" && params["action"] == "copy" {
            if !params.as_object().is_some_and(|fields| fields.len() == 2) {
                return Err("invalid_parameters");
            }
            let text = params["text"].as_str().ok_or("invalid_parameters")?;
            govorun_os_runtime::clipboard::copy(text)?;
            return Ok(json!({"accepted":true}));
        }
        let before =
            serde_json::to_value(self.report()?.rows()).map_err(|_| "storage_unavailable")?;
        let reply = if method == "data.diagnostic" {
            self.append(params)?;
            Value::Null
        } else if method == "diagnostics.localEffect" && params["action"] == "clear" {
            if !params.as_object().is_some_and(|fields| fields.len() == 2)
                || params["confirmed"] != true
            {
                return Err("invalid_parameters");
            }
            #[cfg(target_os = "macos")]
            macos::clear(&self.store)?;
            #[cfg(target_os = "windows")]
            windows::clear(&self.store.scope(windows::OWNER))?;
            json!({"accepted":true})
        } else {
            return Err("invalid_parameters");
        };
        let after =
            serde_json::to_value(self.report()?.rows()).map_err(|_| "storage_unavailable")?;
        drop(_guard);
        if before != after {
            if let Some(changed) = self.changed.get() {
                changed();
            }
        }
        Ok(reply)
    }
}
