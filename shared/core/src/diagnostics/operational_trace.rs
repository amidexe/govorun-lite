//! Shared content-free diagnostics and report format. OS adapters provide normalized causes.
//! No request, response, text or audio fields.
#[path="operational_trace/android.rs"]
pub(crate) mod android;
use chrono::{DateTime, SecondsFormat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OperationalTrace {
    operation: Operation,
    outcome: Outcome,
    at_millis: i64,
    duration_millis: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<FailureReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    system_code: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
enum Operation { Connect, RuntimeRequest, WorkerStart, WorkerExit, Record, Cancel, Microphone, Styles, Account, Statistics, Dictionary,
    Starting, Recording, Processing, Idle, Fallback }
#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
enum Outcome { Success, Cancelled, Timeout, Unavailable, Failed }

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum FailureReason { StartFailed, Configuration, Protocol, ModelFiles, Storage, FileMissing, LibraryMissing, LibraryInvalid, Memory, Access, Disconnected }

impl OperationalTrace {
    pub fn new(operation: &str, error: Option<&str>, at_millis: i64, duration_millis: u64) -> Option<Self> {
        let operation = match operation {
            "runtime.connect" => Operation::Connect,
            "runtime.snapshot" => Operation::RuntimeRequest,
            "recording.toggle" => Operation::Record,
            "recording.cancel" => Operation::Cancel,
            "dictation.action" => Operation::Microphone,
            "styles.action" => Operation::Styles,
            "account.action" => Operation::Account,
            "statistics.snapshot" => Operation::Statistics,
            "dictionary.action" => Operation::Dictionary,
            _ => return None,
        };
        let outcome = match error {
            None => Outcome::Success,
            Some("cancelled" | "native_closing") => Outcome::Cancelled,
            Some("native_timeout" | "request_timeout") => Outcome::Timeout,
            Some("native_unavailable" | "native_not_configured" | "native_start_failed") => Outcome::Unavailable,
            Some(_) => Outcome::Failed,
        };
        let reason = match error {
            Some("native_start_failed") => Some(FailureReason::StartFailed),
            Some("native_not_configured" | "invalid_worker_arguments") => Some(FailureReason::Configuration),
            Some("native_identity_mismatch" | "native_protocol_mismatch" | "native_update_required") => Some(FailureReason::Protocol),
            Some("model_files_missing_or_damaged" | "model_full_installer_required") => Some(FailureReason::ModelFiles),
            Some("model_storage_unavailable" | "account_storage_unavailable") => Some(FailureReason::Storage),
            _ => None,
        };
        let trace = Self { operation, outcome, at_millis, duration_millis, reason, system_code: None };
        (trace.useful() && (0..=253_402_300_799_999).contains(&at_millis) && duration_millis <= 600_000).then_some(trace)
    }

    pub fn worker_start_failed(at_millis: i64, code: Option<i32>, reason: FailureReason) -> Self {
        Self { operation: Operation::WorkerStart, outcome: Outcome::Failed, at_millis,
            duration_millis: 0, reason: Some(reason), system_code: code }
    }

    pub fn worker_exit(at_millis: i64, code: Option<i32>, reason: FailureReason) -> Self {
        Self { operation: Operation::WorkerExit, outcome: Outcome::Failed, at_millis,
            duration_millis: 0, reason: Some(reason), system_code: code }
    }
    fn useful(&self) -> bool {
        !matches!(self.operation, Operation::Starting | Operation::Recording | Operation::Processing | Operation::Idle | Operation::Fallback)
            && match self.outcome { Outcome::Success => matches!(self.operation, Operation::Connect), Outcome::Cancelled => false, _ => true }
    }

    pub fn row(&self, occurrence: usize) -> Option<Value> {
        if !self.useful() || !(0..=253_402_300_799_999).contains(&self.at_millis) || self.duration_millis > 600_000 { return None; }
        let date = DateTime::from_timestamp_millis(self.at_millis)?.to_rfc3339_opts(SecondsFormat::Millis, true);
        let summary = match self.operation {
            Operation::Connect if matches!(self.outcome, Outcome::Success) => "Обработчик приложения запущен",
            Operation::Connect => "Не удалось запустить обработчик приложения",
            Operation::RuntimeRequest => "Обработчик приложения не ответил",
            Operation::WorkerStart => "Система не смогла запустить обработчик приложения",
            Operation::WorkerExit => "Обработчик приложения неожиданно остановился",
            Operation::Record => "Команда записи", Operation::Cancel => "Отмена записи",
            Operation::Microphone => "Настройка записи", Operation::Styles => "Настройка улучшения",
            Operation::Account => "Операция аккаунта", Operation::Statistics => "Загрузка статистики",
            Operation::Dictionary => "Операция словаря", Operation::Starting => "Подготовка записи",
            Operation::Recording => "Запись началась", Operation::Processing => "Обработка записи",
            Operation::Idle => "Обработчик ожидает", Operation::Fallback => "Обработка завершена без улучшения",
        };
        let result = match self.outcome { Outcome::Success => "Успешно", Outcome::Cancelled => "Отменено",
            Outcome::Timeout => "Превышено время ожидания", Outcome::Unavailable => "Обработчик недоступен",
            Outcome::Failed => "Ошибка операции" };
        let result = match self.reason {
            Some(FailureReason::StartFailed) => "Система не смогла запустить обработчик",
            Some(FailureReason::Configuration) => "Обработчик отсутствует или неправильно настроен",
            Some(FailureReason::Protocol) => "Версии приложения и обработчика несовместимы",
            Some(FailureReason::ModelFiles) => "Файлы модели отсутствуют или повреждены",
            Some(FailureReason::Storage) => "Не удалось открыть или записать служебное хранилище",
            Some(FailureReason::FileMissing) => "Не найден необходимый файл",
            Some(FailureReason::LibraryMissing) => "Не найдена библиотека или одна из её зависимостей",
            Some(FailureReason::LibraryInvalid) => "Библиотека или исполняемый файл повреждены либо несовместимы",
            Some(FailureReason::Memory) => "Недостаточно памяти или системных ресурсов",
            Some(FailureReason::Access) => "Система отказала в доступе",
            Some(FailureReason::Disconnected) => "Связь с обработчиком прервана",
            None => result,
        };
        let details = if matches!(self.operation, Operation::WorkerStart) {
            format!("{result}{}", self.system_code.map(|code| format!(" · Код системы: {code}")).unwrap_or_default())
        } else if matches!(self.operation, Operation::WorkerExit) {
            format!("{result}{}", self.system_code.map(|code| format!(" · Код завершения: 0x{:08X}", code as u32)).unwrap_or_default())
        } else { format!("{result} · {}: {} мс", if matches!(self.operation, Operation::Connect) {"Запуск"} else {"Ожидание ответа"}, self.duration_millis) };
        let identity = Sha256::digest(format!("{date}:{summary}:{}:{occurrence}", self.duration_millis).as_bytes());
        let hex: String = identity[..16].iter().map(|byte| format!("{byte:02x}")).collect();
        let id = format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]);
        Some(json!({"id": id, "date": date, "level": if matches!(self.outcome, Outcome::Success) {"info"} else if matches!(self.operation, Operation::Connect | Operation::WorkerStart | Operation::WorkerExit) {"error"} else {"warning"},
            "category": "Расширенная диагностика", "summary": summary,
            "details": details, "timing": null}))
    }
}
