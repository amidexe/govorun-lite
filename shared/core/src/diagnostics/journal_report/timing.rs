//! Strict content-free timing records; wire order is preserved for support exports.
use std::collections::BTreeMap;

pub type Fields<'a> = BTreeMap<&'a str, &'a str>;
const REQUIRED: &[&str] = &[
    "elapsed_ms",
    "deadline_ms",
    "local_ms",
    "start_ms",
    "wait_ms",
    "finalize_ms",
    "starts",
    "polls",
    "transport_errors",
    "transport_timeouts",
    "http_errors",
    "phase",
    "result",
];
const OPTIONAL: &[&str] = &[
    "self_hosted",
    "full_ms", "tail_ms", "recognition_ms", "delivery_prep_ms", "processing_ms", "insertion_ms",
    "audio_ms", "offline_chars", "offline_words", "sent_chars", "output_chars", "model_load_ms", "model_cached",
    "last_http",
    "last_response_ms",
    "timeout_limit_ms",
    "server_elapsed_ms",
    "server_admission_ms",
    "server_queue_ms",
    "server_forward_ms",
    "server_finalize_ms",
    "a1_ms",
    "a2_ms",
    "a1_http",
    "a2_http",
    "access_http",
    "access_attempts",
    "on_demand_issues",
];
const RESULTS: &[&str] = &[
    "success",
    "client_deadline",
    "admission_deadline",
    "round_trip_deadline",
    "confirmed_offline",
    "server_timeout",
    "http_timeout",
    "http_error",
    "transport_timeout",
    "transport_error",
    "cancelled",
    "server_error",
    "local_error",
    "reserve_empty",
    "issue_failed",
    "recovery_pending",
    "usage_unavailable",
    "daily_limit",
    "period_limit",
    "access_required",
    "account_changed",
    "pass_rejected",
    "server_busy",
    "server_unavailable",
    "invalid_response",
];
const ATTEMPT_RESULTS: &[&str] = &[
    "success",
    "http_error",
    "transport_error",
    "transport_timeout",
    "round_trip_deadline",
    "admission_deadline",
    "client_deadline",
    "cancelled",
    "local_error",
];
pub(super) const STAGES: &[(&str, &str)] = &[
    ("scheduled", "до начала соединения"),
    ("transport", "сетевой обмен"),
    ("dns", "поиск адреса сервера"),
    ("connect", "установка соединения"),
    ("tls", "защищённое соединение"),
    ("send", "отправка запроса"),
    ("headers", "ожидание заголовков"),
    ("body", "чтение ответа"),
    ("complete", "ответ прочитан"),
];

pub fn parse(value: &str) -> Option<Fields<'_>> {
    if value.len() > 1024 {
        return None;
    }
    let mut fields = Fields::new();
    for part in value.strip_prefix("timing_v1 ")?.split(' ') {
        let (key, text) = part.split_once('=')?;
        if fields.contains_key(key) {
            return None;
        }
        let allowed = match key {
            "access_stage" => ["key","issue","return","retire","allowance"].contains(&text),
            "access_reason" => ["http","timeout","connection","storage","request","response","bridge","exhausted","local","issuance_capacity_unavailable","reserve_not_prepared"].contains(&text),
            "phase" => ["local", "access", "start", "wait", "finalize"].contains(&text),
            "result" => RESULTS.contains(&text),
            "timeout_scope" => ["overall", "admission", "round_trip"].contains(&text),
            "server_sample" => ["processing", "completed", "failed"].contains(&text),
            "a1_stage" | "a2_stage" => STAGES.iter().any(|(stage, _)| *stage == text),
            "a1_result" | "a2_result" => ATTEMPT_RESULTS.contains(&text),
            "model_cached" | "self_hosted" => ["0", "1"].contains(&text),
            _ => {
                (REQUIRED.contains(&key) || OPTIONAL.contains(&key))
                    && digits(text, if extended(key) { 7 } else { 6 })
                    && text.parse::<u32>().ok().is_some_and(|n| {
                        n <= (if extended(key) { 3_600_000 } else { 120_000 })
                            && (!key.ends_with("http") || (100..=599).contains(&n))
                    })
            }
        };
        if !allowed {
            return None;
        }
        fields.insert(key, text);
    }
    if fields.contains_key("access_stage") != fields.contains_key("access_reason")
        || (fields.contains_key("access_stage") && !fields.get("result").is_some_and(|result|
            ["issue_failed","usage_unavailable","daily_limit","period_limit"].contains(result)))
        || (fields.contains_key("access_attempts") && !fields.contains_key("access_stage"))
        || (fields.contains_key("access_http") && fields.get("access_reason")!=Some(&"http"))
        || REQUIRED.iter().any(|key| !fields.contains_key(key))
        || fields.contains_key("server_sample") != fields.contains_key("server_elapsed_ms")
        || fields.contains_key("timeout_scope") != fields.contains_key("timeout_limit_ms")
        || fields.get("timeout_limit_ms") == Some(&"0")
        || [
            "server_admission_ms",
            "server_queue_ms",
            "server_forward_ms",
            "server_finalize_ms",
        ]
        .iter()
        .any(|key| fields.contains_key(key) && !fields.contains_key("server_elapsed_ms"))
    {
        return None;
    }
    for index in [1, 2] {
        let prefix = format!("a{index}_");
        if fields.keys().any(|key| key.starts_with(&prefix))
            && ["ms", "stage", "result"]
                .iter()
                .any(|suffix| !fields.contains_key(format!("{prefix}{suffix}").as_str()))
        {
            return None;
        }
    }
    if fields.contains_key("a2_ms") && !fields.contains_key("a1_ms") {
        return None;
    }
    if fields.contains_key("full_ms") {
        let total = fields["full_ms"].parse::<u32>().ok()?;
        let sum = DELIVERY.iter().try_fold(0u32, |sum, key| sum.checked_add(fields.get(key)?.parse().ok()?))?;
        if sum != total { return None; }
    } else if DELIVERY.iter().any(|key| fields.contains_key(key)) { return None; }
    Some(fields)
}
const DELIVERY: &[&str] = &["tail_ms", "recognition_ms", "delivery_prep_ms", "processing_ms", "insertion_ms"];
fn extended(key: &str) -> bool { key == "full_ms" || key == "audio_ms" || DELIVERY.contains(&key) }
pub(super) fn digits(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| byte.is_ascii_digit())
}
