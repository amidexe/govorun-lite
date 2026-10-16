//! Per-application overrides remain device-local; edits share catalog validation.
use super::choices::{self, ApplicationRule, Context};
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashSet;
fn identity(value: &str) -> bool {
    !value.trim().is_empty()
        && value.encode_utf16().count() <= 512
        && !value.bytes().any(|b| b.is_ascii_control())
}
pub(super) fn admit(rules: &[ApplicationRule]) -> Result<()> {
    let mut ids = HashSet::new();
    if rules.len() > 100
        || rules.iter().any(|rule| {
            !identity(&rule.application_id)
                || !identity(&rule.application_name)
                || !ids.insert(&rule.application_id)
                || rule
                    .emoji_mode
                    .as_deref()
                    .is_some_and(|mode| !matches!(mode, "off" | "restrained" | "lively" | "bright"))
        })
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}
fn meaningful(rule: &ApplicationRule) -> bool {
    rule.style_value.is_some()
        || rule.emoji_mode.is_some()
        || rule.emoji_enabled.is_some()
        || rule.reduced_punctuation_enabled.is_some()
}
fn finish(context: Context, before: Value, rules: Vec<ApplicationRule>) -> Result<Value> {
    admit(&rules)?;
    let mut reply = choices::project(context)?;
    let after = json!(rules);
    reply["write"] = json!(before != after);
    reply["applicationRules"] = after;
    // Rules are device-local, not content or portable preference intent.
    reply["markDirty"] = json!(false);
    Ok(reply)
}
pub(super) fn save(
    context: Context,
    mut rules: Vec<ApplicationRule>,
    mut rule: ApplicationRule,
) -> Result<Value> {
    admit(&rules)?;
    if !identity(&rule.application_id) || !identity(&rule.application_name) {
        return Err(Error::InvalidRequest);
    }
    let choices = choices::project(context.clone())?;
    if let Some(selected) = &rule.style_value {
        if selected != "off"
            && choices["projection"]["optionsBySelection"]
                .get(selected)
                .is_none()
        {
            return Err(Error::InvalidRequest);
        }
    }
    if let Some(mode) = &rule.emoji_mode {
        if !matches!(mode.as_str(), "off" | "restrained" | "lively" | "bright")
            || rule
                .emoji_enabled
                .is_some_and(|enabled| enabled != (mode != "off"))
        {
            return Err(Error::InvalidRequest);
        }
        rule.emoji_enabled = Some(mode != "off");
    }
    let before = json!(rules);
    if let Some(index) = rules
        .iter()
        .position(|value| value.application_id == rule.application_id)
    {
        if meaningful(&rule) {
            rules[index] = rule;
        } else {
            rules.remove(index);
        }
    } else if meaningful(&rule) {
        rules.push(rule);
    }
    finish(context, before, rules)
}
pub(super) fn delete(
    context: Context,
    mut rules: Vec<ApplicationRule>,
    id: String,
    confirmed: bool,
) -> Result<Value> {
    admit(&rules)?;
    if !confirmed || !identity(&id) {
        return Err(Error::InvalidRequest);
    }
    let index = rules
        .iter()
        .position(|rule| rule.application_id == id)
        .ok_or(Error::InvalidRequest)?;
    let before = json!(rules);
    rules.remove(index);
    finish(context, before, rules)
}
