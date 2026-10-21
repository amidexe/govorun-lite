//! Exact content-free diagnostic events admitted from feature workers.
use crate::{Error, Result};
use serde_json::Value;
#[derive(Debug, PartialEq, Eq)]
pub enum Input {
    Failure {
        name: &'static str,
        status: Option<u16>,
        reason: Option<&'static str>,
    },
    CaptureFailure(String),
    InsertionFailure(&'static str),
    Timing(String),
    CaptureTiming(String),
    RecognitionCompleted,
}
pub fn parse(value: &Value) -> Result<Input> {
    let fields = value.as_object().ok_or(Error::InvalidRequest)?;
    let event = fields.get("event").and_then(Value::as_str);
    if fields.len() != 2 && !(fields.len() == 3
        && (event == Some("audio_capture_failed") && fields.contains_key("native")
            || event.is_some_and(|name| SYNC_FAILURES.contains(&name)) && fields.contains_key("reason"))) {
        return Err(Error::InvalidRequest);
    }
    let name = fields
        .get("event")
        .and_then(Value::as_str)
        .ok_or(Error::InvalidRequest)?;
    if name == "asr_completed" {
        return if fields.get("detail").and_then(Value::as_str) == Some("") {
            Ok(Input::RecognitionCompleted)
        } else {Err(Error::InvalidRequest)};
    }
    if name == "audio_capture_failed" {
        let native = fields.get("native").map(|value| serde_json::from_value(value.clone())
            .map_err(|_| Error::InvalidRequest)).transpose()?;
        return super::capture_failure::encode_detail(fields.get("code").and_then(Value::as_str).ok_or(Error::InvalidRequest)?, native)
            .map(Input::CaptureFailure).ok_or(Error::InvalidRequest);
    }
    if name == "insertion_failed" {
        return super::insertion::code(fields.get("code").and_then(Value::as_str).ok_or(Error::InvalidRequest)?)
            .map(Input::InsertionFailure).ok_or(Error::InvalidRequest);
    }
    if name == "audio_capture_ready" {
        let detail = fields
            .get("detail")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidRequest)?;
        if !super::capture_startup::valid_detail(detail) {
            return Err(Error::InvalidRequest);
        }
        return Ok(Input::CaptureTiming(detail.into()));
    }
    if name == "stylization_timing" {
        let timing = fields
            .get("timing")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidRequest)?;
        if super::timing::parse(timing).is_none() {
            return Err(Error::InvalidRequest);
        }
        return Ok(Input::Timing(timing.into()));
    }
    let name = match name {
        "content_sync_failed" => "content_sync_failed",
        "preferences_sync_failed" => "preferences_sync_failed",
        "stats_sync_failed" => "stats_sync_failed",
        "account_sync_driver_failed" => "account_sync_driver_failed",
        "recording_feedback_failed" => "recording_feedback_failed",
        "self_host_request_failed" => "self_host_request_failed",
        _ => return Err(Error::InvalidRequest),
    };
    let value = fields.get("status").ok_or(Error::InvalidRequest)?;
    let status = if value.is_null() {
        None
    } else {
        Some(
            value
                .as_u64()
                .filter(|value| (100..=599).contains(value))
                .ok_or(Error::InvalidRequest)? as u16,
        )
    };
    let reason = match fields.get("reason") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .and_then(sync_reason)
                .filter(|_| status.is_none() && SYNC_FAILURES.contains(&name))
                .ok_or(Error::InvalidRequest)?,
        ),
    };
    Ok(Input::Failure { name, status, reason })
}

const SYNC_FAILURES: &[&str] = &[
    "content_sync_failed",
    "preferences_sync_failed",
    "stats_sync_failed",
    "account_sync_driver_failed",
];
/// Closed sync failure codes kept when the server sent no HTTP status, so a
/// row says whether the network, storage or a response failed. No content.
pub const SYNC_REASONS: &[(&str, &str)] = &[
    ("sync_transport_failed", "Нет связи с сервером"),
    ("sync_invalid_response", "Сервер вернул неожиданный ответ"),
    ("sync_invalid_request", "Неверный запрос синхронизации"),
    ("sync_storage_unavailable", "Локальные данные недоступны"),
    ("sync_storage_invalid", "Локальные данные не прочитаны"),
    ("sync_stage_panicked", "Внутренняя ошибка синхронизации"),
    ("sync_conflict", "Данные изменились на другом устройстве"),
    ("account_invalid_response", "Сервер аккаунтов вернул неожиданный ответ"),
    ("account_signed_out", "Вход в аккаунт не выполнен"),
    ("account_consent_unconfirmed", "Согласие не подтверждено"),
    ("documents_invalid_configuration", "Синхронизация не настроена"),
];
pub fn sync_reason(code: &str) -> Option<&'static str> {
    SYNC_REASONS.iter().map(|(known, _)| *known).find(|known| *known == code)
}
pub fn sync_reason_title(code: &str) -> Option<&'static str> {
    SYNC_REASONS.iter().find(|(known, _)| *known == code).map(|(_, title)| *title)
}

pub fn self_host_failure_message(status: Option<u16>) -> String {
    match status.filter(|value| (100..=599).contains(value)) {
        Some(status) => format!("Свой сервер: HTTP {status}."),
        None => "Свой сервер: обработка не выполнена.".into(),
    }
}
