use super::{
    lookup, privacy, support, system, timing, windows_catalog as catalog, Row, WindowsEvent,
    MAXIMUM,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub(super) fn rows(events: &[WindowsEvent]) -> Vec<Row> {
    let mut occurrences = HashMap::<String, u32>::new();
    events.iter().filter_map(|event| {
        if !super::valid_date(&event.date) || !super::valid_date(&event.identity_date) { return None; }
        let detail = privacy::windows(&event.name, &event.detail)?;
        let timing = event.timing.as_deref().filter(|text| timing::parse(text).is_some());
        let identity = format!("{}\n{}\n{}\n{}", event.identity_date, event.name, detail, timing.unwrap_or(""));
        let ordinal = occurrences.entry(identity.clone()).or_default();
        let digest = Sha256::digest(format!("{identity}\n{ordinal}").as_bytes()); *ordinal += 1;
        // .NET Guid's first three fields interpret the hash in little-endian order.
        let id = format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            digest[3],digest[2],digest[1],digest[0],digest[5],digest[4],digest[7],digest[6],digest[8],digest[9],digest[10],digest[11],digest[12],digest[13],digest[14],digest[15]);
        let trace = timing.or_else(|| (event.name == "stylization_timing").then_some(detail.as_str())).filter(|text| timing::parse(text).is_some());
        let fields = trace.and_then(timing::parse);
        let local = fields.as_ref().is_some_and(super::delivery::local_only);
        let action = trace.is_some();
        let skip = if trace.is_none() {super::delivery::skipped(&event.name)} else {None};
        let failed = fields.as_ref().is_some_and(|fields| !["success", "cancelled"].contains(&fields["result"]));
        let level = if event.name.ends_with("_failed") { "error" } else if problem(&event.name) || failed { "warning" } else { "info" };
        let summary = if let Some(fields) = fields {
            super::delivery::summary(&fields)
        } else if skip.is_some() {"Распознавание завершено".into()}
        else if event.name == "insertion_failed" && super::insertion::code(&detail).is_some() { super::insertion::failure_summary() }
        else { lookup(catalog::TITLES, &event.name).unwrap_or("Событие приложения").into() };
        let details = trace.map(|text| support::windows("stylization_timing", text))
            .unwrap_or_else(|| skip.map(str::to_owned).unwrap_or_else(||event_detail(&event.name, &detail)));
        let details = details.trim();
        Some(Row { id, date: event.date.clone(), level: if local && !failed || skip.is_some() {"info"} else {level}.into(), category: if action || skip.is_some() {"Распознавание"} else {category(&event.name)}.into(), summary,
            details: (!details.is_empty()).then(|| details.into()), timing: trace.map(str::to_owned), compact: trace.and_then(timing::parse).and_then(|f| super::delivery::compact(&f)) })
    }).take(MAXIMUM).collect()
}
fn problem(name: &str) -> bool {
    name == "style_pass_token_rejected"
        || name.ends_with("_empty")
        || name == "stylization_skipped_long"
        || [
            "failed",
            "fallback",
            "error",
            "pending",
            "degraded",
            "unavailable",
            "deferred",
        ]
        .iter()
        .any(|part| name.contains(part))
}
fn category(name: &str) -> &'static str {
    match name.split('_').next().unwrap_or("") {
        "audio" | "recording" => "Запись",
        "asr" | "recognition" | "dictation" | "short" => "Распознавание",
        "stylization" | "style" | "self" | "credit" | "online" => "Стили",
        "account" | "content" | "sync" | "preferences" => "Аккаунт",
        "billing" | "tbank" => "Подписка",
        "paste" | "insertion" => "Вставка",
        "dictionary" => "Словарь",
        "stats" | "statistics" => "Статистика",
        _ => "Приложение",
    }
}
fn event_detail(name: &str, detail: &str) -> String {
    if name == "self_host_request_failed" {
        let http = detail.strip_prefix("http_").map(|status| format!("HTTP {status}. ")).unwrap_or_default();
        return format!("{http}Использован исходный текст.");
    }
    if name == "audio_capture_failed" {
        if let Some(detail) = super::capture_failure::failure_detail(detail) { return detail; }
    }
    if name == "insertion_failed" {
        if let Some(code) = super::insertion::code(detail) { return super::insertion::failure_detail(code); }
    }
    if let Some(value) = system::display(detail) {
        return value;
    }
    if name == "self_hosted_stylization" {
        let Some((result, elapsed)) = detail.split_once(' ') else {
            return String::new();
        };
        let title = match result {
            "result=success" => "Текст обработан",
            "result=unchanged" => "Текст без изменений",
            "result=auth_failed" => "Проверьте ключ и доступ к модели. Использован исходный текст",
            "result=cancelled" => "Запрос отменён",
            _ => "Ошибка сервера. Использован исходный текст",
        };
        return format!(
            "{title} · {} мс",
            elapsed.strip_prefix("elapsed_ms=").unwrap_or("")
        );
    }
    if name == "stylization_timing"
        || name == "stylization_fallback" && support::reason(detail).is_some()
    {
        return support::windows(name, detail);
    }
    if let Some(title) = super::input::sync_reason_title(detail) {
        return format!("{title} ({detail})");
    }
    if ["http_500", "http_502", "http_503", "http_504"].contains(&detail) {
        return format!(
            "Ошибка сервиса. Повторите проверку позже (HTTP {})",
            &detail[5..]
        );
    }
    match lookup(catalog::DETAIL_TITLES, detail) {
        Some(title) => title.into(),
        None if super::input::code(detail) => format!("Код: {detail}"),
        None => String::new(),
    }
}
