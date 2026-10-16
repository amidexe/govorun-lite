//! Closed OS error facts: no exception message, paths or arbitrary data.
use super::{lookup, timing::digits};
const STAGES: &[(&str, &str)] = &[
    ("application_start", "Запуск приложения"),
    ("model_prepare", "Загрузка модели распознавания"),
    ("microphone_permission", "Проверка доступа к микрофону"),
    ("vad_prepare", "Подготовка обработки звука"),
    ("device_open", "Открытие микрофона"),
    ("first_audio", "Ожидание звука от микрофона"),
    ("audio_capture", "Получение звука"),
    ("recording_start", "Начало записи"),
];
const REASONS: &[(&str, &str)] = &[
    ("memory", "Недостаточно памяти или системных ресурсов"),
    ("disk_full", "Недостаточно места на диске"),
    (
        "library_missing",
        "Не найдена библиотека или одна из её зависимостей",
    ),
    (
        "library_invalid",
        "Библиотека повреждена или несовместима с приложением",
    ),
    (
        "entry_missing",
        "В библиотеке отсутствует необходимая функция",
    ),
    ("access_denied", "Система отказала в доступе"),
    ("device_unavailable", "Устройство ввода недоступно"),
    ("device_busy", "Устройство ввода занято другим приложением"),
    ("audio_service", "Служба звука Windows не запущена"),
    ("timeout", "Превышено время ожидания"),
    ("file_missing", "Не найден необходимый файл"),
    ("io", "Не удалось прочитать или записать данные"),
    ("cancelled", "Операция отменена"),
    ("unknown", "Причина не определена; сохранён системный код"),
];

pub(super) fn display(value: &str) -> Option<String> {
    if value.len() > 160 {
        return None;
    }
    let parts: Vec<_> = value.strip_prefix("system_v1 ")?.split(' ').collect();
    let [stage, reason, hresult, win32] = parts.as_slice() else {
        return None;
    };
    let stage = lookup(STAGES, stage.strip_prefix("stage=")?)?;
    let reason = lookup(REASONS, reason.strip_prefix("reason=")?)?;
    let hresult = hresult.strip_prefix("hresult=")?;
    let win32 = win32.strip_prefix("win32=")?;
    if !digits(hresult, 10) || !digits(win32, 10) {
        return None;
    }
    let hresult: u32 = hresult.parse().ok()?;
    let _: u32 = win32.parse().ok()?;
    Some(
        format!("Этап: {stage}\n{reason}\nHRESULT: 0x{hresult:08X}")
            + &if win32 == "0" {
                String::new()
            } else {
                format!("\nКод Windows: {win32}")
            },
    )
}

/// The adapter unwraps its OS error and supplies only its type and numeric facts.
/// Messages, paths, exception Data and stack traces cannot enter this boundary.
#[derive(Clone, Copy)]
pub enum FailureKind {
    Memory,
    LibraryMissing,
    LibraryInvalid,
    EntryMissing,
    AccessDenied,
    Timeout,
    FileMissing,
    Io,
    Cancelled,
    Unknown,
}
impl FailureKind {
    fn reason(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::LibraryMissing => "library_missing",
            Self::LibraryInvalid => "library_invalid",
            Self::EntryMissing => "entry_missing",
            Self::AccessDenied => "access_denied",
            Self::Timeout => "timeout",
            Self::FileMissing => "file_missing",
            Self::Io => "io",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
        }
    }
}
/// Preserve the original Windows classifier without relying on exception text.
pub fn describe(stage: &str, kind: FailureKind, hresult: u32, native_code: Option<u32>) -> String {
    let stage = if lookup(STAGES, stage).is_some() {
        stage
    } else {
        "application_start"
    };
    let win32 = native_code.unwrap_or_else(|| {
        if hresult & 0xffff0000 == 0x80070000 {
            hresult & 0xffff
        } else {
            0
        }
    });
    let reason = match win32 {
        8 | 14 | 1455 => "memory",
        39 | 112 => "disk_full",
        5 => "access_denied",
        126 => "library_missing",
        193 | 216 => "library_invalid",
        127 => "entry_missing",
        _ => kind.reason(),
    };
    let reason = match hresult {
        0x88890004 => "device_unavailable",
        0x8889000a => "device_busy",
        0x88890010 => "audio_service",
        _ => reason,
    };
    format!("system_v1 stage={stage} reason={reason} hresult={hresult} win32={win32}")
}
