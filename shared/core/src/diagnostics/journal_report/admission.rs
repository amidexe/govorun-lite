//! Frozen privacy and bounded journal-write rules shared by physical stores.
use super::{macos_catalog, privacy, timing, MacEvent, WindowsEvent, MAXIMUM};

pub fn macos(mut event: MacEvent) -> Option<MacEvent> {
    if !super::valid_id(&event.id)
        || !super::valid_date(&event.date)
        || !["info", "warning", "error"].contains(&event.level.as_str())
    {
        return None;
    }
    event.message = privacy::macos(&event.message, &event.level);
    if !macos_catalog::CATEGORIES.contains(&event.category.as_str()) {
        event.category = "Приложение".into();
    }
    event.timing = event.timing.filter(|value| timing::parse(value).is_some());
    (!event.message.is_empty() || event.timing.is_some()).then_some(event)
}
pub fn windows(mut event: WindowsEvent) -> Option<WindowsEvent> {
    if !super::valid_date(&event.date) || !super::valid_date(&event.identity_date) {
        return None;
    }
    event.detail = privacy::windows(&event.name, &event.detail)?;
    event.timing = event.timing.filter(|value| timing::parse(value).is_some());
    Some(event)
}
fn bounded<T>(mut events: Vec<T>) -> Vec<T> {
    if events.len() > MAXIMUM {
        events.drain(..events.len() - MAXIMUM);
    }
    events
}
fn preparation_failure(message: &str) -> bool {
    message.starts_with("Подготовка допуска:")
        || message.starts_with("Фоновая подготовка отложена;")
        || [
            "Проверка цифровой подписи токена доступа не пройдена.",
            "Не удалось создать защищённый токен доступа.",
            "Не удалось подготовить токен доступа.",
            "Состояние лимита будет обновлено повторно.",
            "Состояние стилизации после восстановления связи будет обновлено повторно.",
        ]
        .contains(&message)
}
pub fn should_record_background(
    message: &str,
    previous: impl DoubleEndedIterator<Item = impl AsRef<str>>,
) -> bool {
    if !preparation_failure(message) {
        return true;
    }
    for earlier in previous.rev() {
        let earlier = earlier.as_ref();
        if [
            "Токены доступа к стилизации получены.",
            "Приложение запущено.",
        ]
        .contains(&earlier)
        {
            return true;
        }
        if preparation_failure(earlier) {
            return earlier != message;
        }
    }
    true
}
/// Storage is chronological; distinct attempts intentionally bypass coalescing.
pub fn append_macos(events: Vec<MacEvent>, event: MacEvent, deduplicate: bool) -> Vec<MacEvent> {
    let mut events: Vec<_> = bounded(events.into_iter().filter_map(macos).collect());
    let Some(event) = macos(event) else {
        return events;
    };
    if deduplicate {
        if event.category == "Стили"
            && !should_record_background(&event.message, events.iter().map(|event| &event.message))
        {
            return events;
        }
        if let Some(last) = events.last() {
            if last.category == event.category
                && last.level == event.level
                && last.message == event.message
            {
                let date =
                    chrono::DateTime::parse_from_rfc3339(&event.date).expect("admitted date");
                let previous =
                    chrono::DateTime::parse_from_rfc3339(&last.date).expect("admitted date");
                if date.signed_duration_since(previous) < chrono::Duration::seconds(60) {
                    return events;
                }
            }
        }
    }
    events.push(event);
    if events.len() > MAXIMUM {
        events.drain(..events.len() - MAXIMUM);
    }
    events
}
/// Windows storage has no duplicate suppression and cleans legacy fields on write.
pub fn append_windows(events: Vec<WindowsEvent>, event: WindowsEvent) -> Vec<WindowsEvent> {
    let mut events: Vec<_> = bounded(events.into_iter().filter_map(windows).collect());
    if let Some(event) = windows(event) {
        events.push(event);
        if events.len() > MAXIMUM {
            events.drain(..events.len() - MAXIMUM);
        }
    }
    events
}
