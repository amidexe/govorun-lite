use super::{active, frame, validate, Options, State};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const BUILT_INS: [&str; 3] = ["formal", "light", "concise"];

/// Stored option maps and legacy mirrors are facts. Selection, defaults and
/// updates have the same meaning on every binding.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Context {
    pub(super) state: State,
    pub(super) built_in_options: BTreeMap<String, Options>,
    pub(super) current_options: Options,
    pub(super) current_options_style: String,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum OptionName {
    EmojiMode,
    ReducedPunctuation,
    ProfanityFilter,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SelectionOptions {
    pub(super) emoji_mode: String,
    pub(super) reduced_punctuation: bool,
    pub(super) profanity_filter: bool,
}

fn valid_options(options: &Options) -> bool {
    matches!(options.punctuation_mode.as_str(), "normal" | "less")
        && matches!(
            options.emoji_mode.as_str(),
            "off" | "restrained" | "lively" | "bright"
        )
}

pub(super) fn admit(context: &Context) -> Result<()> {
    validate(&context.state)?;
    if !matches!(
        context.current_options_style.as_str(),
        "formal" | "light" | "concise" | "custom"
    ) || !valid_options(&context.current_options)
        || context
            .built_in_options
            .iter()
            .any(|(id, options)| !BUILT_INS.contains(&id.as_str()) || !valid_options(options))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

pub(super) fn selection(state: &State) -> Option<String> {
    if state.selected_style == "custom" {
        active(state).map(|preset| format!("custom:{}", preset.id))
    } else {
        Some(state.selected_style.clone())
    }
}

pub(super) fn option_for(context: &Context, selection: &str) -> Option<Options> {
    if let Some(id) = selection.strip_prefix("custom:") {
        return context
            .state
            .presets
            .iter()
            .find(|preset| preset.id == id)
            .map(Options::from_style);
    }
    if !BUILT_INS.contains(&selection) {
        return None;
    }
    Some(
        context
            .built_in_options
            .get(selection)
            .cloned()
            .unwrap_or_else(|| {
                if context.current_options_style == selection {
                    context.current_options.clone()
                } else {
                    Options::defaults()
                }
            }),
    )
}

fn reply(original: &Context, next: Context, mutation: bool, reason: &str) -> Value {
    let selected = selection(&next.state);
    let selected_options = selected.as_deref().and_then(|id| option_for(&next, id));
    let mut options = BTreeMap::new();
    for id in BUILT_INS {
        options.insert(id.to_string(), option_for(&next, id).unwrap());
    }
    for preset in &next.state.presets {
        options.insert(format!("custom:{}", preset.id), Options::from_style(preset));
    }
    let mut result = frame(&original.state, next.state.clone(), mutation, reason, None);
    let changed = original.state != next.state
        || original.built_in_options != next.built_in_options
        || original.current_options != next.current_options;
    result["write"] = json!(mutation && changed);
    let content_changed = original.state.presets != next.state.presets
        || (original.state.selected_style == "custom")
            .then(|| selection(&original.state))
            .flatten()
            != (next.state.selected_style == "custom")
                .then(|| selection(&next.state))
                .flatten();
    result["markDirty"] = json!(mutation && content_changed);
    result["builtInOptions"] = json!(next.built_in_options);
    result["currentOptions"] = json!(next.current_options);
    result["currentOptionsStyle"] = json!(next.current_options_style);
    result["projection"]["selected"] = json!(selected);
    result["projection"]["selectedOptions"] = json!(selected_options);
    result["projection"]["optionsBySelection"] = json!(options);
    result
}

pub(super) fn project(context: Context) -> Result<Value> {
    admit(&context)?;
    Ok(reply(&context, context.clone(), false, "ok"))
}

pub(super) fn select(
    context: Context,
    selected: String,
    activation: Option<SelectionOptions>,
) -> Result<Value> {
    admit(&context)?;
    let Some(mut options) = option_for(&context, &selected) else {
        return Ok(reply(&context, context.clone(), false, "missingPreset"));
    };
    if let Some(activation) = activation {
        options = Options {
            punctuation_mode: if activation.reduced_punctuation {
                "less"
            } else {
                "normal"
            }
            .into(),
            emoji_mode: activation.emoji_mode,
            profanity_filter_enabled: activation.profanity_filter,
        };
        if !valid_options(&options) {
            return Err(Error::InvalidRequest);
        }
    }
    let mut next = context.clone();
    if let Some(id) = selected.strip_prefix("custom:") {
        super::select(&mut next.state, id);
        options.apply(
            next.state
                .presets
                .iter_mut()
                .find(|preset| preset.id == id)
                .ok_or(Error::InvalidRequest)?,
        )?;
    } else {
        next.state.selected_style = selected.clone();
        if option_for(&context, &selected).as_ref() != Some(&options) {
            next.built_in_options.insert(selected, options.clone());
        }
    }
    if next.state.selected_style != context.state.selected_style
        && context.state.selected_style == context.current_options_style
        && BUILT_INS.contains(&context.current_options_style.as_str())
    {
        next.built_in_options
            .entry(context.current_options_style.clone())
            .or_insert_with(|| context.current_options.clone());
    }
    next.current_options = options;
    next.current_options_style = next.state.selected_style.clone();
    Ok(reply(&context, next, true, "ok"))
}

pub(super) fn set_option(context: Context, name: OptionName, value: Value) -> Result<Value> {
    admit(&context)?;
    let Some(selected) = selection(&context.state) else {
        return Ok(reply(&context, context.clone(), false, "missingPreset"));
    };
    let mut options = option_for(&context, &selected).ok_or(Error::InvalidRequest)?;
    match name {
        OptionName::EmojiMode => {
            options.emoji_mode = value.as_str().ok_or(Error::InvalidRequest)?.into()
        }
        OptionName::ReducedPunctuation => {
            options.punctuation_mode = if value.as_bool().ok_or(Error::InvalidRequest)? {
                "less"
            } else {
                "normal"
            }
            .into()
        }
        OptionName::ProfanityFilter => {
            options.profanity_filter_enabled = value.as_bool().ok_or(Error::InvalidRequest)?
        }
    }
    if !valid_options(&options) {
        return Err(Error::InvalidRequest);
    }
    let mut next = context.clone();
    if let Some(id) = selected.strip_prefix("custom:") {
        let preset = next
            .state
            .presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .ok_or(Error::InvalidRequest)?;
        options.apply(preset)?;
    } else {
        next.built_in_options.insert(selected, options.clone());
    }
    next.current_options = options;
    next.current_options_style = next.state.selected_style.clone();
    Ok(reply(&context, next, true, "ok"))
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApplicationRule {
    pub(super) application_id: String,
    pub(super) application_name: String,
    pub(super) style_value: Option<String>,
    pub(super) emoji_enabled: Option<bool>,
    pub(super) emoji_mode: Option<String>,
    pub(super) reduced_punctuation_enabled: Option<bool>,
}

pub(super) fn detach(mut rules: Vec<ApplicationRule>, id: String) -> Result<Value> {
    if rules.len() > 100 || !super::valid_id(&id) {
        return Err(Error::InvalidRequest);
    }
    let selection = format!("custom:{id}");
    for rule in &mut rules {
        if rule.style_value.as_deref() == Some(&selection) {
            rule.style_value = None;
        }
    }
    rules.retain(|rule| {
        rule.style_value.is_some()
            || rule.emoji_enabled.is_some()
            || rule.emoji_mode.is_some()
            || rule.reduced_punctuation_enabled.is_some()
    });
    Ok(json!(rules))
}
