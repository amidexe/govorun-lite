use super::{lookup, macos_catalog as mac, support, system, timing, windows_catalog as win};
use regex::Regex;
use std::sync::OnceLock;

pub(super) fn macos(input: &str, level: &str) -> String {
    if super::capture_failure::from_stored(input).is_some() { return input.into(); }
    if super::insertion::from_stored(input).is_some() { return input.into(); }
    if input.is_empty() || input == "Событие приложения." {
        return String::new();
    }
    if let Some(value) = lookup(mac::LEGACY_MESSAGES, input) {
        return value.into();
    }
    if timing::parse(input).is_some()
        || mac::FIXED.contains(&input)
        || mac::SUMMARIES.iter().any(|(_, text)| *text == input)
    {
        return input.into();
    }
    static MEASURED: OnceLock<Vec<Regex>> = OnceLock::new();
    if input.chars().count() <= 200
        && MEASURED
            .get_or_init(|| {
                mac::MEASURED
                    .iter()
                    .map(|pattern| Regex::new(pattern).expect("closed journal pattern"))
                    .collect()
            })
            .iter()
            .any(|pattern| pattern.is_match(input))
    {
        return input.into();
    }
    for prefix in support::REASON_PREFIXES {
        if input
            .strip_prefix(prefix)
            .and_then(|text| text.strip_suffix('.'))
            .is_some_and(|reason| mac::REASONS.contains(&reason))
        {
            return input.into();
        }
    }
    if input.starts_with("Стилизация выполнена:") {
        return "Стилизация выполнена.".into();
    }
    for (prefix, summary) in mac::SUMMARIES {
        if input.starts_with(prefix) {
            return (*summary).into();
        }
    }
    match level {
        "error" => "Не удалось выполнить операцию.",
        "warning" => "Операция требует внимания.",
        _ => "",
    }
    .into()
}
pub(super) fn windows(name: &str, input: &str) -> Option<String> {
    if name == "self_host_request_failed" {
        return Some(input.strip_prefix("http_").filter(|value| timing::digits(value, 3)).and_then(|value| value.parse::<u16>().ok())
            .filter(|value| (100..=599).contains(value)).map(|_| input.to_owned()).unwrap_or_default());
    }
    if name == "audio_capture_failed" && super::capture_failure::valid_detail(input) { return Some(input.into()); }
    if name == "insertion_failed" && super::insertion::code(input).is_some() { return Some(input.into()); }
    if name == "self_hosted_stylization" {
        let parts: Vec<_> = input.split(' ').collect();
        let allowed = parts.len() == 2
            && [
                "result=success",
                "result=unchanged",
                "result=auth_failed",
                "result=cancelled",
                "result=failed",
            ]
            .contains(&parts[0])
            && parts[1]
                .strip_prefix("elapsed_ms=")
                .is_some_and(|number| timing::digits(number, 9));
        return Some(if allowed { input.into() } else { String::new() });
    }
    if name == "stylization_timing" {
        return Some(if timing::parse(input).is_some() {
            input.into()
        } else {
            String::new()
        });
    }
    if name == "unknown_event" || !win::EVENTS.contains(&name) {
        return None;
    }
    if [
        "application_start_failed",
        "asr_model_prepare_failed",
        "recording_start_failed",
        "audio_capture_failed",
    ]
    .contains(&name)
        && system::display(input).is_some()
        || win::DETAILS.contains(&input)
        || super::input::code(input)
    {
        return Some(input.into());
    }
    if input.encode_utf16().count() > 512 {
        return Some(String::new());
    }
    Some(
        input
            .split(';')
            .filter(|part| {
                part.split_once('=').is_some_and(|(key, value)| {
                    win::METRICS.contains(&key) && timing::digits(value, 9)
                })
            })
            .take(10)
            .collect::<Vec<_>>()
            .join(";"),
    )
}
