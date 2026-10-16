use super::{privacy, support, timing, MacEvent, Row, MAXIMUM};

pub(super) fn rows(events: &[MacEvent]) -> Vec<Row> {
    events.iter().filter_map(row).rev().take(MAXIMUM).collect()
}
fn row(event: &MacEvent) -> Option<Row> {
    if !super::valid_id(&event.id)
        || !super::valid_date(&event.date)
        || !["info", "warning", "error"].contains(&event.level.as_str())
    {
        return None;
    }
    let message = privacy::macos(&event.message, &event.level);
    let trace = event
        .timing
        .as_deref()
        .filter(|text| timing::parse(text).is_some())
        .or_else(|| timing::parse(&message).map(|_| message.as_str()));
    let category = if super::macos_catalog::CATEGORIES.contains(&event.category.as_str()) {
        event.category.as_str()
    } else {
        "Приложение"
    };
    let local = trace.and_then(timing::parse).is_some_and(|f| super::delivery::local_only(&f));
    let action = trace.is_some();
    let failed = trace.and_then(timing::parse).is_some_and(|f| !["success","cancelled"].contains(&f["result"]));
    let skip = if trace.is_none() {super::delivery::skipped(&message)} else {None};
    let (summary, details) = if let Some(fields) = trace.and_then(timing::parse) {
        let summary = super::delivery::summary(&fields);
        let details = support::macos(trace?);
        (summary, Some(details))
    } else if let Some(reason) = skip {
        ("Распознавание завершено".into(),Some(reason.into()))
    } else if let Some(code) = super::capture_failure::from_stored(&message) {
        ("Не удалось начать запись.".into(), super::capture_failure::failure_detail(code))
    } else if let Some(code) = super::insertion::from_stored(&message) {
        (super::insertion::failure_summary(), Some(super::insertion::failure_detail(code)))
    } else if category == "Подписка"
        && [
            "Не удалось проверить промокод:",
            "Не удалось подтвердить применение промокода:",
        ]
        .iter()
        .any(|prefix| message.starts_with(prefix))
    {
        let (summary, details) = message.split_once(": ")?;
        (summary.into(), Some(details.into()))
    } else {
        (support::macos(&message), None)
    };
    if summary.is_empty() {
        return None;
    }
    Some(Row {
        id: event.id.clone(),
        date: event.date.clone(),
        level: if local && !failed || skip.is_some() {"info".into()} else if failed && event.level == "info" {"warning".into()} else {event.level.clone()},
        category: if action || skip.is_some() {"Распознавание"} else {category}.into(),
        summary,
        details,
        timing: trace.map(str::to_owned),
        compact: trace
            .and_then(timing::parse)
            .and_then(|f| super::delivery::compact(&f)),
    })
}
