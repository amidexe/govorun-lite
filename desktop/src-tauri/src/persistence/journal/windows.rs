//! The chronological tab-separated journal of the old app, no user/account fields. Since
//! 2.1.112 it is a row of the documents base, moved once from `diagnostics.log`.
use crate::persistence::file_documents::Disk;
use chrono::{Datelike, SecondsFormat, Timelike};
use govorun_core::journal_report::{admission, WindowsEvent};
/// The journal's view of the base owns no settings row.
pub const OWNER: &str = "journal";
const FILE: &str = "diagnostics.log";
const LIMIT: usize = 8 * 1_048_576;
fn parse(line: &str) -> Option<WindowsEvent> {
    let mut fields = line.split('\t');
    let timestamp = chrono::DateTime::parse_from_rfc3339(fields.next()?.trim()).ok()?;
    if !(1..=9999).contains(&timestamp.year()) || timestamp.nanosecond() >= 1_000_000_000 {
        return None;
    }
    let name = fields.next()?.into();
    let detail = fields.next().unwrap_or("").into();
    let timing = fields.next().map(str::to_owned);
    let identity_date = format!(
        "{}.{:07}{}",
        timestamp.format("%Y-%m-%dT%H:%M:%S"),
        timestamp.nanosecond() / 100,
        timestamp.format("%:z")
    );
    admission::windows(WindowsEvent {
        date: timestamp
            .to_utc()
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        identity_date,
        name,
        detail,
        timing,
    })
}
/// Sanitized storage order; presentation reverses it exactly once at the port.
pub fn read(store: &impl Disk) -> Result<Vec<WindowsEvent>, &'static str> {
    let Some(raw) = store.read(FILE, LIMIT).map_err(|_| "storage_unavailable")? else {
        return Ok(vec![]);
    };
    let mut events: Vec<_> = raw
        .trim_start_matches('\u{feff}')
        .lines()
        .filter_map(parse)
        .collect();
    if events.len() > 200 {
        events.drain(..events.len() - 200);
    }
    Ok(events)
}
pub fn append(store: &impl Disk, event: WindowsEvent) -> Result<(), &'static str> {
    if admission::windows(event.clone()).is_none() {
        return Err("invalid_parameters");
    }
    let events = admission::append_windows(read(store)?, event);
    let text = events
        .into_iter()
        .map(|event| {
            let mut line = format!("{}\t{}\t{}", event.identity_date, event.name, event.detail);
            if let Some(timing) = event.timing {
                line.push('\t');
                line.push_str(&timing);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    store.write(FILE, &text).map_err(|_| "storage_unavailable")
}
pub fn clear(store: &impl Disk) -> Result<(), &'static str> {
    store.write(FILE, "").map_err(|_| "storage_unavailable")
}
