//! Project Android's existing closed outcome and timing through the common report.
use super::{delivery, lookup, support, support_catalog, timing::Fields};
use serde_json::{json, Value};

fn legacy(reason: &str) -> &'static str {
    match reason {
        "STYLE_PASS_RESERVE_EMPTY" => "reserve_empty",
        "STYLE_PASS_ISSUE_FAILED" => "issue_failed",
        "STYLE_PASS_RECOVERY_PENDING" => "recovery_pending",
        "ACCOUNT_STATE_CHANGED" => "account_changed",
        "TRANSPORT_FAILED" => "transport_error",
        "SERVER_TIMEOUT" => "legacy_unclassified_timeout",
        "OFFLINE" | "CONFIRMED_OFFLINE" => "confirmed_offline",
        "SERVER_BUSY" => "server_busy",
        "SERVER_UNAVAILABLE" | "SERVICE_UNAVAILABLE" => "server_unavailable",
        "INVALID_RESPONSE" => "invalid_response",
        "INSERTION_FAILED" => "insertion_failed",
        "NO_CHANGE" => "no_change",
        _ => "local_error",
    }
}
pub(crate) fn present(mut fields: Option<Fields<'_>>, outcome: Option<&str>, reason: Option<&str>) -> Value {
    let fallback = outcome == Some("FALLBACK");
    let skipped = outcome == Some("SKIPPED");
    let insertion = fallback && reason == Some("INSERTION_FAILED");
    if let Some(f) = fields.as_mut() {
        // Old Android records may have a default success trace despite a real
        // pre-request failure. The measured result wins whenever it is specific.
        if fallback && !insertion && f["result"] == "success" {
            f.insert("result", legacy(reason.unwrap_or("")));
        }
        let success = f["result"] == "success" && !fallback;
        let mut details = delivery::report(f);
        if insertion { details += "\nКод: GV-I01. Не удалось вставить текст."; }
        return json!({"title":delivery::summary(f),"details":details,
            "shortSummary":delivery::compact(f),"category":"Распознавание",
            "attention":(!success && f["result"] != "cancelled") || insertion,"success":success});
    }
    let success = !fallback && (skipped || outcome == Some("STYLED"));
    let result = legacy(reason.unwrap_or(""));
    let detail = if fallback {
        let code = support::result(result).map(|code|format!("Код: {code}. ")).unwrap_or_default();
        let label = if result == "legacy_unclassified_timeout" {
            "Превышено время ожидания; источник уточняется в замере".to_owned()
        } else {
            let label = lookup(support_catalog::LABELS, result).unwrap_or("локальный отказ");
            let mut chars=label.chars(); chars.next().map(|c|c.to_uppercase().to_string()+chars.as_str()).unwrap_or_default()
        };
        let stage=if insertion {"Вставка"} else {"Стилизация"};
        Some(format!("{stage}. {code}{label}. Использован исходный текст."))
    } else if success && !skipped {Some("Стилизация выполнена.".into())} else { None };
    json!({"title":"Распознавание завершено",
        "details":detail,"shortSummary":null,"category":"Распознавание",
        "attention":fallback,"success":success})
}
