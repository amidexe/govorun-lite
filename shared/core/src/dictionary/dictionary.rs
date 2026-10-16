use crate::{dictionary_document as document, Result};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Command {
    Parse {
        raw: String,
    },
    ReadStorage {
        stored: Value,
    },
    WriteStorage {
        raw: String,
        enabled: bool,
    },
    ImportStable {
        entries: Value,
        portable: Option<Value>,
        enabled: bool,
    },
    Edit {
        raw: String,
        entries: Vec<document::Entry>,
    },
    Apply {
        raw: String,
        text: String,
        enabled: bool,
    },
    Normalize {
        text: String,
    },
    Variants {
        text: String,
    },
    AppendVariants {
        existing: Vec<String>,
        recognized: String,
        correct: String,
    },
    VoiceMappings {
        raw: String,
        correct: String,
        captured: Vec<String>,
    },
    TextMapping {
        raw: String,
        recognized: String,
        replacement: String,
    },
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    use crate::{dictionary_replacement as replacement, dictionary_voice as voice};
    Ok(match command {
        Command::Parse { raw } => json!({"entries": document::parse(&raw)?}),
        Command::ReadStorage { stored } => crate::dictionary_storage::read(stored)?,
        Command::WriteStorage { raw, enabled } => crate::dictionary_storage::write(raw, enabled)?,
        Command::ImportStable { entries, portable, enabled } =>
            crate::dictionary_storage::read_split(entries, portable, enabled)?,
        Command::Edit { raw, entries } => json!({"raw": document::edit(&raw, entries)?}),
        Command::Apply { raw, text, enabled } => {
            json!({"text": replacement::apply(&raw, &text, enabled)?})
        }
        Command::Normalize { text } => json!({"text": voice::normalize(&text)}),
        Command::Variants { text } => json!({"variants": voice::variants(&text)}),
        Command::AppendVariants {
            existing,
            recognized,
            correct,
        } => json!({"variants": voice::append(existing, &recognized, &correct)}),
        Command::VoiceMappings {
            raw,
            correct,
            captured,
        } => {
            document::parse(&raw)?;
            let raw = voice::mappings(&raw, &correct, captured);
            document::parse(&raw)?;
            json!({"raw":raw})
        }
        Command::TextMapping {
            raw,
            recognized,
            replacement,
        } => {
            document::parse(&raw)?;
            let raw = voice::text_mapping(&raw, &recognized, &replacement);
            document::parse(&raw)?;
            json!({"raw":raw})
        }
    })
}
