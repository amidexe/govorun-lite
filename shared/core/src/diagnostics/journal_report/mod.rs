//! Common content-free view of existing native journals. No I/O or network.
pub mod admission;
pub(crate) mod android_presentation;
pub mod capture_startup;
pub(crate) mod delivery;
pub mod input;
pub mod insertion;
mod macos;
mod macos_catalog;
mod privacy;
pub(crate) mod support;
mod support_catalog;
pub mod system;
pub mod capture_failure;
pub mod timing;
mod windows;
mod windows_catalog;
use serde::{Deserialize, Serialize};

pub const MAXIMUM: usize = 200;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub id: String,
    pub date: String,
    pub level: String,
    pub category: String,
    pub summary: String,
    pub details: Option<String>,
    pub timing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<String>"))]
    pub compact: Option<String>,
}
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "platform",
    content = "entries",
    rename_all = "lowercase",
    deny_unknown_fields
)]
pub enum Journal {
    Macos(Vec<MacEvent>),
    Windows(Vec<WindowsEvent>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MacEvent {
    pub id: String,
    pub date: String,
    pub level: String,
    pub category: String,
    pub message: String,
    pub timing: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsEvent {
    pub date: String,
    pub identity_date: String,
    pub name: String,
    pub detail: String,
    pub timing: Option<String>,
}
impl Journal {
    pub fn rows(&self) -> Vec<Row> {
        match self {
            Self::Macos(entries) => macos::rows(entries),
            Self::Windows(entries) => windows::rows(entries),
        }
    }
    pub fn export(&self) -> String {
        self.export_recent(MAXIMUM)
    }
    pub fn export_recent(&self, maximum: usize) -> String {
        let mut rows = self.rows();
        rows.truncate(maximum.min(MAXIMUM));
        let windows = matches!(self, Self::Windows(_));
        if !windows {
            rows.reverse();
        }
        let separator = if windows { "\r\n" } else { "\n" };
        rows.iter()
            .map(|row| {
                let mut text = if windows {
                    format!("{} · {} · {}", row.date, row.category, row.summary)
                } else {
                    format!(
                        "{} [{}] {}: {}",
                        row.date, row.level, row.category, row.summary
                    )
                };
                for value in [&row.details, &row.timing].into_iter().flatten() {
                    text.push_str(separator);
                    text.push_str(value);
                }
                text
            })
            .collect::<Vec<_>>()
            .join(separator)
    }
}
fn lookup(table: &'static [(&str, &str)], key: &str) -> Option<&'static str> {
    table
        .iter()
        .find_map(|(name, value)| (*name == key).then_some(*value))
}
fn valid_date(value: &str) -> bool {
    value.len() <= 40 && chrono::DateTime::parse_from_rfc3339(value).is_ok()
}
fn valid_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, byte)| {
            if [8, 13, 18, 23].contains(&i) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}
