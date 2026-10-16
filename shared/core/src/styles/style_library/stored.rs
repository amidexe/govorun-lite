use super::{active, choices, validate, Options, State};
use crate::{style_definition::Style, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// OS storage facts include absent preferences on a fresh install. Reading them
/// does not migrate or rewrite storage.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Stored {
    presets: Option<Vec<StoredPreset>>,
    active_id: Option<String>,
    selected_style: Option<String>,
    instruction: String,
    options_by_style: BTreeMap<String, StoredOptions>,
    legacy_options: StoredOptions,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredPreset {
    id: String,
    name: String,
    instruction: String,
    #[serde(flatten)]
    options: StoredOptions,
}

#[derive(Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredOptions {
    punctuation_mode: Option<String>,
    reduced_punctuation: Option<bool>,
    emoji_enabled: Option<bool>,
    emoji_mode: Option<String>,
    profanity_filter_enabled: Option<bool>,
}

fn punctuation(raw: &Option<String>) -> Option<&str> {
    match raw.as_deref() {
        Some("less") => Some("less"),
        Some("normal" | "more") => Some("normal"),
        _ => None,
    }
}

fn emoji(raw: &Option<String>) -> Option<&str> {
    raw.as_deref()
        .filter(|mode| matches!(*mode, "off" | "restrained" | "lively" | "bright"))
}

fn inherited(local: &StoredOptions, legacy: &StoredOptions) -> Options {
    Options {
        punctuation_mode: punctuation(&local.punctuation_mode)
            .or_else(|| punctuation(&legacy.punctuation_mode))
            .unwrap_or(if legacy.reduced_punctuation == Some(true) {
                "less"
            } else {
                "normal"
            })
            .into(),
        emoji_mode: emoji(&local.emoji_mode)
            .or_else(|| emoji(&legacy.emoji_mode))
            .unwrap_or(
                if local.emoji_enabled.or(legacy.emoji_enabled) == Some(true) {
                    "bright"
                } else {
                    "off"
                },
            )
            .into(),
        profanity_filter_enabled: local
            .profanity_filter_enabled
            .or(legacy.profanity_filter_enabled)
            .unwrap_or(false),
    }
}

fn preset(value: StoredPreset, inherited: &Options) -> Style {
    let emoji_mode = emoji(&value.options.emoji_mode)
        .unwrap_or_else(|| {
            // A missing mode with true was historically bright. False did not
            // override the inherited mode; retain that effective stored behavior.
            if value.options.emoji_enabled == Some(true) {
                "bright"
            } else {
                &inherited.emoji_mode
            }
        })
        .to_string();
    Style {
        id: value.id,
        name: value.name,
        instruction: value.instruction,
        punctuation_mode: punctuation(&value.options.punctuation_mode)
            .unwrap_or(&inherited.punctuation_mode)
            .into(),
        emoji_enabled: emoji_mode != "off",
        emoji_mode,
        profanity_filter_enabled: value
            .options
            .profanity_filter_enabled
            .unwrap_or(inherited.profanity_filter_enabled),
    }
}

pub(super) fn read(stored: Stored) -> Result<Value> {
    if stored.instruction.len() > 64 * 1024
        || stored
            .options_by_style
            .keys()
            .any(|key| !matches!(key.as_str(), "formal" | "light" | "concise" | "custom"))
        || stored
            .presets
            .as_ref()
            .is_some_and(|presets| presets.len() > 1000)
    {
        return Err(Error::InvalidRequest);
    }
    let options_present = std::iter::once(&stored.legacy_options)
        .chain(stored.options_by_style.values())
        .any(|options| options.emoji_enabled.is_some()
            || options.emoji_mode.is_some()
            || options.profanity_filter_enabled.is_some());
    let selected_style = match stored.selected_style.as_deref() {
        Some(style @ ("formal" | "light" | "concise" | "custom")) => style,
        _ => "formal",
    }
    .to_string();
    let options_for = |style: &str| {
        inherited(
            stored
                .options_by_style
                .get(style)
                .unwrap_or(&StoredOptions::default()),
            &stored.legacy_options,
        )
    };
    let custom_options = options_for("custom");
    let mut state = State {
        presets: Vec::new(),
        active_id: stored.active_id,
        selected_style: selected_style.clone(),
        instruction: stored.instruction,
    };
    state.presets = stored.presets.unwrap_or_default()
        .into_iter()
        .map(|value| preset(value, &custom_options))
        .collect();
    validate(&state)?;
    let current_options = if selected_style == "custom" {
        active(&state)
            .map(Options::from_style)
            .unwrap_or(custom_options)
    } else {
        options_for(&selected_style)
    };
    let context = choices::Context {
        state,
        current_options,
        current_options_style: selected_style,
        built_in_options: ["formal", "light", "concise"]
            .into_iter()
            .map(|style| (style.into(), options_for(style)))
            .collect(),
    };
    let mut result = choices::project(context.clone())?;
    result["context"] = json!(context);
    result["storedOptionsPresent"] = json!(options_present);
    Ok(result)
}
