use super::choices::{self, Context, SelectionOptions};
use crate::{Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

/// Storage presence and local consent are facts, not financial authority.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Facts {
    local_terms_version: u32,
    options_present: bool,
}

pub(super) fn evaluate(
    context: Context,
    facts: Facts,
    supplied: Option<SelectionOptions>,
) -> Result<Value> {
    choices::project(context.clone())?;
    let options = if let Some(options) = supplied {
        if !matches!(options.emoji_mode.as_str(), "off" | "restrained" | "lively" | "bright") {
            return Err(Error::InvalidRequest);
        }
        Some(options)
    } else if facts.local_terms_version == 0
        && !facts.options_present
        && context.state.selected_style == "formal"
    {
        Some(SelectionOptions {
            emoji_mode: "lively".into(),
            reduced_punctuation: context.current_options.punctuation_mode == "less",
            profanity_filter: true,
        })
    } else {
        None
    };
    Ok(json!({"activationOptions": options, "write": false, "markDirty": false}))
}
