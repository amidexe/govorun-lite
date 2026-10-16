use super::{
    lookup, support_catalog as catalog,
    timing,
};

pub(crate) fn result(value: &str) -> Option<&'static str> {
    lookup(catalog::CODES, value)
}
pub(crate) fn reason(value: &str) -> Option<&'static str> {
    result(lookup(catalog::LEGACY, value).unwrap_or(value))
}
pub(super) fn label(value: &str) -> &str {
    lookup(catalog::LABELS, value).unwrap_or(value)
}
pub(super) const REASON_PREFIXES: &[&str] = &[
    "Онлайн-стилизация недоступна; локальный текст вставлен; причина: ",
    "Онлайн-стилизация недоступна; сохранён исходный текст; причина: ",
];

pub(super) fn macos(message: &str) -> String {
    if let Some(fields) = timing::parse(message) {
        return super::delivery::report(&fields);
    }
    for prefix in REASON_PREFIXES {
        if let Some(code) = message
            .strip_prefix(prefix)
            .and_then(|text| text.strip_suffix('.'))
            .and_then(reason)
        {
            return format!("Код: {code}. {message}");
        }
    }
    message.to_owned()
}
pub(super) fn windows(name: &str, detail: &str) -> String {
    if name == "stylization_timing" {
        if let Some(fields) = timing::parse(detail) {
            return super::delivery::report(&fields);
        }
    }
    if name == "stylization_fallback" {
        let code = reason(detail)
            .map(|code| format!("Код: {code}. "))
            .unwrap_or_default();
        return code + label(lookup(catalog::LEGACY, detail).unwrap_or(detail));
    }
    detail.to_owned()
}
