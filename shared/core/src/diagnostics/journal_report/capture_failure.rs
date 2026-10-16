//! Closed capture failures; OS messages, device names and audio are never stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation { Format, Open, Start }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause { Unavailable, Unsupported, InvalidArgument, Backend, Other }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    pub operation: Operation,
    pub cause: Cause,
    pub os_code: Option<i32>,
}
const CODES: &[(&str, &str, &str)] = &[
    ("microphone_permission_required", "Доступ", "Система не разрешила запись голоса"),
    ("microphone_unavailable", "Открытие", "Микрофон недоступен"),
    ("microphone_device_busy", "Открытие", "Микрофон занят другим приложением"),
    ("microphone_audio_service_unavailable", "Открытие", "Служба звука Windows не запущена"),
    ("microphone_open_failed", "Открытие", "Не удалось открыть аудиопоток"),
    ("microphone_start_failed", "Запуск", "Не удалось запустить аудиопоток"),
    ("microphone_format_unavailable", "Формат", "Не удалось получить формат микрофона"),
    ("microphone_format_unsupported", "Формат", "Формат микрофона не поддерживается"),
    ("microphone_invalid_configuration", "Открытие", "Аудиосистема отклонила параметры потока"),
    ("microphone_callback_failed", "Запись", "Аудиопоток завершился с ошибкой"),
    ("microphone_timeout", "Первый звук", "Микрофон не прислал звук за время ожидания"),
    ("microphone_timestamp_unavailable", "Запись", "Не удалось определить время звука"),
    ("microphone_packet_invalid", "Запись", "Микрофон прислал некорректный блок звука"),
    ("microphone_buffer_overflow", "Запись", "Переполнен буфер звука"),
    ("recording_start_failed", "Запуск", "Не удалось начать запись"),
];
pub fn code(value: &str) -> Option<&'static str> { CODES.iter().find(|row| row.0 == value).map(|row| row.0) }
pub fn encode_detail(value: &str, native: Option<Native>) -> Option<String> {
    let code = code(value)?;
    Some(match native {
        Some(native) => format!("{code} {}", serde_json::to_string(&native).ok()?),
        None => code.into(),
    })
}
fn parse(value: &str) -> Option<(&'static str, Option<Native>)> {
    if value.len() > 256 { return None; }
    match value.split_once(' ') {
        Some((name, native)) => Some((code(name)?, Some(serde_json::from_str(native).ok()?))),
        None => Some((code(value)?, None)),
    }
}
pub fn valid_detail(value: &str) -> bool { parse(value).is_some() }
pub fn stored(value: &str) -> Option<String> { parse(value).map(|_| format!("capture_failure {value}")) }
pub fn from_stored(value: &str) -> Option<&str> {
    let detail = value.strip_prefix("capture_failure ")?;
    valid_detail(detail).then_some(detail)
}
pub fn failure_detail(value: &str) -> Option<String> {
    let (code, native) = parse(value)?;
    let (_, stage, reason) = CODES.iter().find(|row| row.0 == code)?;
    let mut detail = format!("{reason}. Этап: {stage}. Код: {code}.");
    if let Some(native) = native {
        let operation = match native.operation { Operation::Format => "формат", Operation::Open => "открытие", Operation::Start => "запуск" };
        let cause = match native.cause {
            Cause::Unavailable => "устройство недоступно", Cause::Unsupported => "формат не поддерживается",
            Cause::InvalidArgument => "неверные параметры", Cause::Backend => "отказ аудиосистемы", Cause::Other => "внутренняя ошибка",
        };
        detail.push_str(&format!(" CPAL: {operation}, {cause}."));
        if let Some(code) = native.os_code { detail.push_str(&format!(" Код системы: 0x{:08X}.", code as u32)); }
    }
    Some(detail)
}
