use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) const MAX_BYTES: usize = 240 * 1024;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub id: String,
    pub from: String,
    pub to: String,
}

struct Line<'a> {
    body: &'a str,
    ending: &'a str,
    entry: Option<Entry>,
    unusable: bool,
}

pub(crate) fn pair(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (from, to) = line
        .split_once(" → ")
        .or_else(|| line.split_once('='))
        .unwrap_or((line, ""));
    let from = from.trim();
    (!from.is_empty()).then_some((from, to.trim()))
}

/// A rule line this version cannot use stays in the text untouched and is skipped;
/// it never makes the whole dictionary unreadable. Writers check new lines strictly.
fn lines(raw: &str) -> Vec<Line<'_>> {
    let mut result = Vec::new();
    let (mut entries, mut variants) = (0, 0);
    let mut offset = 0;
    while offset < raw.len() {
        let end = raw[offset..]
            .find(['\r', '\n'])
            .map(|n| n + offset)
            .unwrap_or(raw.len());
        let next = if raw[end..].starts_with("\r\n") {
            end + 2
        } else if end < raw.len() {
            end + 1
        } else {
            end
        };
        let body = &raw[offset..end];
        let candidate = pair(body).map(|(from, to)| {
            // Stable view identity, independent of OS UUID generators. Not a content log.
            let digest = Sha256::digest(format!("{}\0{from}\0{to}", result.len()).as_bytes());
            let hex: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
            Entry {
                id: format!(
                    "{}-{}-{}-{}-{}",
                    &hex[..8],
                    &hex[8..12],
                    &hex[12..16],
                    &hex[16..20],
                    &hex[20..]
                ),
                from: from.to_owned(),
                to: to.to_owned(),
            }
        });
        let entry = candidate.clone().filter(|entry| {
            let count = entry.from.split(',').filter(|v| !v.trim().is_empty()).count();
            let fits = usable(entry) && entries < 5_000 && variants + count <= 5_000;
            if fits {
                entries += 1;
                variants += count;
            }
            fits
        });
        result.push(Line {
            body,
            ending: &raw[end..next],
            unusable: candidate.is_some() && entry.is_none(),
            entry,
        });
        offset = next;
    }
    result
}

fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn usable(e: &Entry) -> bool {
    !e.from.trim().is_empty()
        && e.from.chars().count() <= 500
        && e.to.chars().count() <= 500
        && !e.from.contains(['\r', '\n', '='])
        && !e.from.contains(" → ")
        && !e.to.contains(['\r', '\n'])
}

fn validate_entries(entries: &[Entry]) -> Result<()> {
    let mut ids = HashSet::new();
    let variants: usize = entries
        .iter()
        .map(|e| e.from.split(',').filter(|v| !v.trim().is_empty()).count())
        .sum();
    if entries.len() > 5_000
        || variants > 5_000
        || entries
            .iter()
            .any(|e| !ids.insert(e.id.to_ascii_lowercase()) || !valid_id(&e.id) || !usable(e))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

/// Whether a stored line is kept in the text but skipped; the journal names it once.
pub fn has_unusable(raw: &str) -> bool {
    lines(raw).iter().any(|line| line.unusable)
}

/// A writer may keep lines it found unusable, but never add one.
pub(crate) fn no_new_unusable(before: &str, after: &str) -> Result<()> {
    let unusable = |raw| lines(raw).iter().filter(|line| line.unusable).count();
    if after.len() > MAX_BYTES || unusable(after) > unusable(before) {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

pub(crate) fn parse(raw: &str) -> Result<Vec<Entry>> {
    if raw.len() > MAX_BYTES {
        return Err(Error::ResponseTooLarge);
    }
    let entries: Vec<_> = lines(raw).into_iter().filter_map(|l| l.entry).collect();
    validate_entries(&entries)?;
    Ok(entries)
}

pub(crate) fn edit(raw: &str, entries: Vec<Entry>) -> Result<String> {
    parse(raw)?;
    validate_entries(&entries)?;
    let mut desired = entries.iter();
    let mut result = String::new();
    for line in lines(raw) {
        match line.entry {
            None => {
                result.push_str(line.body);
                result.push_str(line.ending);
            }
            Some(previous) => {
                if let Some(next) = desired.next() {
                    if next.id.eq_ignore_ascii_case(&previous.id)
                        && next.from == previous.from
                        && next.to == previous.to
                    {
                        result.push_str(line.body);
                    } else {
                        result.push_str(&render(next));
                    }
                    result.push_str(line.ending);
                }
            }
        }
    }
    for entry in desired {
        if !result.is_empty() && !result.ends_with(['\n', '\r']) {
            result.push('\n');
        }
        result.push_str(&render(entry));
        result.push('\n');
    }
    parse(&result)?;
    Ok(result)
}

fn render(entry: &Entry) -> String {
    if entry.to.trim().is_empty() {
        format!("{} =", entry.from.trim())
    } else {
        format!("{} = {}", entry.from.trim(), entry.to.trim())
    }
}
