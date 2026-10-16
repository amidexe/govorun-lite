//! Normalize typed recorder failures; retain only enums and a numeric OS code.
//! Driver text and device names never leave this adapter.
pub fn system_code(message: &str) -> Option<i32> {
    if message.len() > 4096 { return None; }
    let lower = message.to_ascii_lowercase();
    if let Some((_, tail)) = lower.rsplit_once("0x") {
        let number = tail.get(..8)?;
        if number.bytes().all(|c| c.is_ascii_hexdigit())
            && !tail.as_bytes().get(8).is_some_and(u8::is_ascii_hexdigit) {
            return u32::from_str_radix(number, 16).ok().map(|value| value as i32);
        }
    }
    let (_, tail) = lower.rsplit_once("os error ")?;
    let number: String = tail.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
    number.parse().ok()
}

pub fn classify(message: &str, fallback: &'static str) -> &'static str {
    match system_code(message).map(|code| code as u32) {
        Some(0x80070005) => return "microphone_permission_required",
        Some(0x8889000a) => return "microphone_device_busy",
        Some(0x88890010) => return "microphone_audio_service_unavailable",
        Some(0x88890004) => return "microphone_unavailable",
        Some(0x88890008) => return "microphone_format_unsupported",
        Some(0x80070057) => return "microphone_invalid_configuration",
        _ => (),
    }
    let message = message.to_ascii_lowercase();
    if message.strip_prefix("a backend-specific error has occurred: ").unwrap_or(&message)
        == "microphone_timestamp_unavailable" {
        "microphone_timestamp_unavailable"
    } else if [
        "access is denied",
        "permission denied",
        "0x80070005",
        "os error -2147024891",
    ]
    .iter()
    .any(|part| message.contains(part))
    {
        "microphone_permission_required"
    } else if message.contains("0x8889000a") || message.contains("os error -2004287478") {
        "microphone_device_busy"
    } else if message.contains("0x88890010") || message.contains("os error -2004287472") {
        "microphone_audio_service_unavailable"
    } else if message.contains("0x88890004") || message.contains("os error -2004287484") {
        "microphone_unavailable"
    } else {
        fallback
    }
}

#[cfg(feature = "capture")]
pub fn recorder(error: &handy_recorder::Error, admission: &super::admission::Admission,
    operation: govorun_core::journal_report::capture_failure::Operation) -> &'static str {
    use handy_recorder::ErrorKind::*;
    use govorun_core::journal_report::capture_failure::{Cause, Native};
    let (code, cause) = match error.kind() {
        PermissionDenied => ("microphone_permission_required", Cause::Backend),
        DeviceUnavailable | DeviceLost => ("microphone_unavailable", Cause::Unavailable),
        DeviceBusy => ("microphone_device_busy", Cause::Backend),
        UnsupportedFormat => ("microphone_format_unsupported", Cause::Unsupported),
        InvalidChannel | StreamInvalidated => ("microphone_invalid_configuration", Cause::InvalidArgument),
        OpenTimedOut | NoAudio | Stalled | SinkStalled | CloseTimedOut => ("microphone_timeout", Cause::Backend),
        _ => ("microphone_callback_failed", Cause::Backend),
    };
    admission.set_native_failure(Some(Native { operation, cause,
        os_code: error.detail().and_then(system_code) }));
    error.detail().map_or(code, |message| classify(message, code))
}
