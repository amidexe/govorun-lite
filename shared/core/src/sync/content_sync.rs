use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use crate::style_definition::Style;

pub(crate) const MAX_CUSTOM_STYLES: usize = crate::style_definition::MAX_PRESETS;

/// Portable content only. No account identity, credentials, IO or migration.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Document {
    schema_version: u32,
    dictionary: Dictionary,
    selected_custom_style_id: String,
    custom_styles: Vec<Style>,
}

impl Default for Document {
    fn default() -> Self {
        Self { schema_version: 1, dictionary: Dictionary { enabled: false, text: String::new() },
            selected_custom_style_id: String::new(), custom_styles: Vec::new() }
    }
}

impl Document {
    pub(crate) fn meaningful(&self) -> Result<bool> {
        validate(self)?;
        Ok(!self.dictionary.text.trim().is_empty() || !self.custom_styles.is_empty())
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct Dictionary { enabled: bool, text: String }

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    ReadResponse { status: u16, body: String, etag: Option<String>, fallback: String },
    Validate { document: Document },
    Merge { base: Document, local: Document, remote: Document },
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    if let Command::ReadResponse { status,body,etag,fallback }=&command {
        let etag=etag.clone().unwrap_or_else(||fallback.clone());
        if *status==304 {return Ok(json!({"configured":false,"etag":etag,"notModified":true,"document":Document::default()}))}
        let value:Value=serde_json::from_str(body).map_err(|_|Error::InvalidResponse)?;
        if value["schemaVersion"]!=1 || !value["configured"].is_boolean() {return Err(Error::InvalidResponse)}
        let document:Document=serde_json::from_value(value["document"].clone()).map_err(|_|Error::InvalidResponse)?;
        validate(&document).map_err(|_|Error::InvalidResponse)?;
        return Ok(json!({"configured":value["configured"],"etag":etag,"notModified":false,"document":document}))
    }
    let document = match command {
        Command::ReadResponse { .. }=>unreachable!(),
        Command::Validate { document } => { validate(&document)?; document }
        Command::Merge { base, local, remote } => {
            for document in [&base, &local, &remote] { validate(document)?; }
            let merged = merge(base, local, remote);
            validate(&merged)?;
            merged
        }
    };
    Ok(json!({"document":document}))
}

pub(crate) fn validate(document: &Document) -> Result<()> {
    crate::dictionary_document::parse(&document.dictionary.text)?;
    let mut ids = HashSet::new();
    let valid = document.schema_version == 1
        && document.dictionary.text.len() <= 240 * 1024
        && document.custom_styles.len() <= MAX_CUSTOM_STYLES
        && document.custom_styles.iter().all(|style| {
            ids.insert(&style.id) && style.valid()
        })
        && (document.selected_custom_style_id.is_empty() || ids.contains(&document.selected_custom_style_id));
    if !valid || serde_json::to_vec(document).map_err(|_| Error::InvalidRequest)?.len() > 256 * 1024 {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

fn value<T: PartialEq>(base: &T, local: T, remote: T) -> T {
    if &local == base { remote } else { local }
}

pub(crate) fn merge(base: Document, local: Document, remote: Document) -> Document {
    let custom_styles = styles(&base.custom_styles, &local.custom_styles, &remote.custom_styles);
    let selected = value(&base.selected_custom_style_id, local.selected_custom_style_id, remote.selected_custom_style_id);
    Document {
        schema_version: 1,
        dictionary: Dictionary {
            enabled: value(&base.dictionary.enabled, local.dictionary.enabled, remote.dictionary.enabled),
            text: dictionary(&base.dictionary.text, &local.dictionary.text, &remote.dictionary.text),
        },
        selected_custom_style_id: if custom_styles.iter().any(|s| s.id == selected) { selected } else { String::new() },
        custom_styles,
    }
}

fn styles(base: &[Style], local: &[Style], remote: &[Style]) -> Vec<Style> {
    let index = |items: &[Style]| items.iter().cloned().map(|s| (s.id.clone(), s)).collect::<HashMap<_, _>>();
    let originals = index(base);
    let left = index(local);
    let right = index(remote);
    let mut seen = HashSet::new();
    remote.iter().chain(local).filter(|s| seen.insert(&s.id)).filter_map(|s| {
        let original = originals.get(&s.id);
        let a = left.get(&s.id);
        let b = right.get(&s.id);
        if (a.is_none() && b == original) || (b.is_none() && a == original) { return None; }
        match (original, a, b) {
            (Some(o), Some(a), Some(b)) => {
                let emoji_mode = value(&o.emoji_mode, a.emoji_mode.clone(), b.emoji_mode.clone());
                Some(Style {
                    id: a.id.clone(),
                    name: value(&o.name, a.name.clone(), b.name.clone()),
                    instruction: value(&o.instruction, a.instruction.clone(), b.instruction.clone()),
                    punctuation_mode: value(&o.punctuation_mode, a.punctuation_mode.clone(), b.punctuation_mode.clone()),
                    emoji_enabled: emoji_mode != "off",
                    emoji_mode,
                    profanity_filter_enabled: value(&o.profanity_filter_enabled, a.profanity_filter_enabled, b.profanity_filter_enabled),
                })
            }
            (_, Some(a), _) => Some(a.clone()),
            (_, _, b) => b.cloned(),
        }
    }).take(MAX_CUSTOM_STYLES).collect()
}

fn lines(text: &str) -> Vec<((String, usize), String)> {
    let mut occurrences = HashMap::new();
    text.replace("\r\n", "\n").replace('\r', "\n").split('\n')
        .map(str::trim).filter(|s| !s.is_empty()).map(|line| {
            let identity = if line.starts_with('#') { format!("comment:{line}") }
                else { format!("rule:{}", line.split('=').next().unwrap_or(line).trim().to_lowercase()) };
            let occurrence = occurrences.entry(identity.clone()).or_insert(0);
            let key = (identity, *occurrence);
            *occurrence += 1;
            (key, line.to_owned())
        }).collect()
}

fn dictionary(base: &str, local: &str, remote: &str) -> String {
    if local == base { return remote.to_owned(); }
    if remote == base || local == remote { return local.to_owned(); }
    let originals: HashMap<_, _> = lines(base).into_iter().collect();
    let a = lines(local);
    let b = lines(remote);
    let left: HashMap<_, _> = a.iter().cloned().collect();
    let right: HashMap<_, _> = b.iter().cloned().collect();
    let mut seen = HashSet::new();
    a.iter().chain(&b).filter(|(id, _)| seen.insert(id)).filter_map(|(id, _)| {
        value(&originals.get(id), left.get(id), right.get(id)).cloned()
    }).collect::<Vec<_>>().join("\n")
}
