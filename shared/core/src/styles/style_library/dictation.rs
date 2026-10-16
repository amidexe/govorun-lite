//! Device-local application rules resolve into the existing request options.
//! This read never changes global selection or emits application identity.
use super::{application_rules, choices, Options};
use crate::{Error, Result};
use serde_json::{json, Value};

// No Debug/Serialize: custom instructions stay inside the processing attempt.
pub struct Resolved {
    pub enabled: bool,
    pub payload: Value,
}
#[derive(Clone, Copy)]
pub enum ApplicationIdentity {
    Exact,
    Windows,
}
pub(crate) fn resolve(
    context: Value,
    rules: Value,
    application: Option<&str>,
    identity: ApplicationIdentity,
) -> Result<Resolved> {
    let context: choices::Context =
        serde_json::from_value(context).map_err(|_| Error::InvalidRequest)?;
    choices::admit(&context)?;
    let rules: Vec<choices::ApplicationRule> =
        serde_json::from_value(rules).map_err(|_| Error::InvalidRequest)?;
    application_rules::admit(&rules)?;
    let application = application.map(|id| match identity {
        ApplicationIdentity::Exact => id.to_owned(),
        ApplicationIdentity::Windows => {
            let name = id.rsplit(['\\', '/']).next().unwrap_or("");
            name.rsplit_once('.')
                .map_or(name, |(stem, _)| stem)
                .trim()
                .to_lowercase()
        }
    });
    let rule = application.as_ref().and_then(|id| {
        rules.iter().find(|rule| match identity {
            ApplicationIdentity::Exact => rule.application_id == *id,
            ApplicationIdentity::Windows => rule.application_id.to_lowercase() == *id,
        })
    });
    resolve_rule(context, rule)
}
pub(super) fn resolve_rule(context: choices::Context, rule: Option<&choices::ApplicationRule>) -> Result<Resolved> {
    let enabled = rule.and_then(|rule| rule.style_value.as_deref()) != Some("off");
    let default = choices::selection(&context.state);
    let selected = if !enabled {
        Some("formal")
    } else {
        rule.and_then(|rule| rule.style_value.as_deref())
            .filter(|selected| choices::option_for(&context, selected).is_some())
            .or(default.as_deref())
    };
    let style = selected
        .filter(|selected| !selected.starts_with("custom:"))
        .unwrap_or(&context.state.selected_style);
    let preset = selected
        .and_then(|selected| selected.strip_prefix("custom:"))
        .and_then(|id| context.state.presets.iter().find(|preset| preset.id == id));
    let mut options: Options = selected
        .and_then(|selected| choices::option_for(&context, selected))
        .unwrap_or_else(|| context.current_options.clone());
    if let Some(rule) = rule {
        if let Some(mode) = &rule.emoji_mode {
            options.emoji_mode = mode.clone();
        } else if let Some(enabled) = rule.emoji_enabled {
            options.emoji_mode = if enabled { "bright" } else { "off" }.into();
        }
        if let Some(reduced) = rule.reduced_punctuation_enabled {
            options.punctuation_mode = if reduced { "less" } else { "normal" }.into();
        }
    }
    let mut payload = json!({"style":if preset.is_some() {"custom"} else {style},
        "punctuationMode":options.punctuation_mode,"reducedPunctuation":options.punctuation_mode=="less",
        "emojiMode":options.emoji_mode,"emojiEnabled":options.emoji_mode!="off",
        "profanityFilterEnabled":options.profanity_filter_enabled});
    if payload["style"] == "custom" {
        payload["instruction"] = json!(preset
            .map_or(context.state.instruction.as_str(), |preset| preset
                .instruction
                .as_str()));
    }
    Ok(Resolved { enabled, payload })
}
