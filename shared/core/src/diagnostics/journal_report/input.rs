//! Exact content-free diagnostic events admitted from feature workers.
use crate::{Error, Result};
use serde_json::Value;
#[derive(Debug, PartialEq, Eq)]
pub enum Input {
    Failure {
        name: &'static str,
        status: Option<u16>,
    },
    CaptureFailure(String),
    InsertionFailure(&'static str),
    Timing(String),
    CaptureTiming(String),
    RecognitionCompleted,
}
pub fn parse(value: &Value) -> Result<Input> {
    let fields = value.as_object().ok_or(Error::InvalidRequest)?;
    if fields.len() != 2 && !(fields.len() == 3
        && fields.get("event").and_then(Value::as_str) == Some("audio_capture_failed")
        && fields.contains_key("native")) {
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
    Ok(Input::Failure { name, status })
}

pub fn self_host_failure_message(status: Option<u16>) -> String {
    match status.filter(|value| (100..=599).contains(value)) {
        Some(status) => format!("Свой сервер: HTTP {status}."),
        None => "Свой сервер: обработка не выполнена.".into(),
    }
}
