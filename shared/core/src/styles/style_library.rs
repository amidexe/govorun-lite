#[path="style_library/android_preferences.rs"]
pub(crate) mod android_preferences;
use crate::{
    style_definition::{valid_id, Style, MAX_INSTRUCTION, MAX_NAME, MAX_PRESETS},
    Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
#[path="style_library/activation_options.rs"]
mod activation_options;
#[path="style_library/application_rules.rs"]
mod application_rules;
#[path="style_library/choices.rs"]
mod choices;
#[path="style_library/dictation.rs"]
pub(crate) mod dictation;
#[path="style_library/stored.rs"]
mod stored;

pub(crate) fn validate_stored_application_rules(value: &Value) -> Result<()> {
    let rules: Vec<choices::ApplicationRule> =
        serde_json::from_value(value.clone()).map_err(|_| Error::InvalidRequest)?;
    application_rules::admit(&rules)
}

/// Local library state. Account authority, styling enablement and IO are separate
/// owners. Reading a view never rewrites the persisted strings or IDs.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct State {
    presets: Vec<Style>,
    active_id: Option<String>,
    selected_style: String,
    instruction: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Options {
    punctuation_mode: String,
    emoji_mode: String,
    profanity_filter_enabled: bool,
}

impl Options {
    fn defaults() -> Self {
        Self {
            punctuation_mode: "normal".into(),
            emoji_mode: "off".into(),
            profanity_filter_enabled: false,
        }
    }
    fn from_style(style: &Style) -> Self {
        Self {
            punctuation_mode: style.punctuation_mode.clone(),
            emoji_mode: style.emoji_mode.clone(),
            profanity_filter_enabled: style.profanity_filter_enabled,
        }
    }
    fn apply(&self, style: &mut Style) -> Result<()> {
        if !matches!(self.punctuation_mode.as_str(), "less" | "normal")
            || !matches!(
                self.emoji_mode.as_str(),
                "off" | "restrained" | "lively" | "bright"
            )
        {
            return Err(Error::InvalidRequest);
        }
        style.punctuation_mode = self.punctuation_mode.clone();
        style.emoji_mode = self.emoji_mode.clone();
        style.emoji_enabled = self.emoji_mode != "off";
        style.profanity_filter_enabled = self.profanity_filter_enabled;
        Ok(())
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) enum Field {
    Name,
    Instruction,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Command {
    ActivationSelection { first_activation: bool, requested: Option<String>, selected: String, available: Vec<String> },
    ReadStored {
        stored: stored::Stored,
    },
    Limits {},
    InspectText {
        field: Field,
        value: String,
    },
    Project {
        state: State,
    },
    Save {
        state: State,
        id: Option<String>,
        new_id: String,
        name: String,
        instruction: String,
        options: Option<Options>,
        select_saved: bool,
    },
    SaveActive {
        state: State,
        new_id: String,
        instruction: String,
        current_options: Options,
    },
    Select {
        state: State,
        id: String,
    },
    Delete {
        state: State,
        id: String,
    },
    Options {
        state: State,
        id: String,
        options: Options,
    },
    Replace {
        state: State,
        presets: Vec<Style>,
        selected_id: String,
    },
    Choices {
        context: choices::Context,
    },
    ActivationOptions {
        context: choices::Context,
        facts: activation_options::Facts,
        supplied: Option<choices::SelectionOptions>,
    },
    SelectStyle {
        context: choices::Context,
        selection: String,
        activation_options: Option<choices::SelectionOptions>,
    },
    SetSelectedOption {
        context: choices::Context,
        name: choices::OptionName,
        value: Value,
    },
    DetachApplicationReferences {
        rules: Vec<choices::ApplicationRule>,
        id: String,
    },
    DeleteWithReferences {
        state: State,
        id: String,
        rules: Vec<choices::ApplicationRule>,
    },
    SaveApplicationRule {
        context: choices::Context,
        rules: Vec<choices::ApplicationRule>,
        rule: choices::ApplicationRule,
    },
    DeleteApplicationRule {
        context: choices::Context,
        rules: Vec<choices::ApplicationRule>,
        id: String,
        confirmed: bool,
    },
}

fn validate(state: &State) -> Result<()> {
    let mut ids = HashSet::new();
    if state.presets.len() > 1000
        || !state
            .presets
            .iter()
            .all(|style| ids.insert(&style.id) && style.admissible_stored())
        || !matches!(
            state.selected_style.as_str(),
            "formal" | "light" | "concise" | "custom"
        )
        || state.instruction.len() > 64 * 1024
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

fn active(state: &State) -> Option<&Style> {
    state
        .presets
        .iter()
        .find(|style| Some(&style.id) == state.active_id.as_ref())
        .or_else(|| {
            (!state.instruction.is_empty())
                .then(|| {
                    state
                        .presets
                        .iter()
                        .find(|style| style.instruction == state.instruction)
                })
                .flatten()
        })
}

fn select(state: &mut State, id: &str) -> bool {
    let Some(style) = state.presets.iter().find(|style| style.id == id) else {
        return false;
    };
    state.active_id = Some(style.id.clone());
    state.instruction = style.instruction.clone();
    state.selected_style = "custom".into();
    true
}

fn limits() -> Value {
    json!({"maxPresets":MAX_PRESETS,"maxNameCharacters":MAX_NAME,
        "minInstructionCharacters":3,"maxInstructionCharacters":MAX_INSTRUCTION})
}

fn frame(
    original: &State,
    state: State,
    dirty: bool,
    reason: &str,
    preset: Option<Style>,
) -> Value {
    let changed = original != &state;
    json!({"state":state,"accepted":reason == "ok","reason":reason,
        "write":changed,"markDirty":changed && dirty,"preset":preset,
        "projection":{"activeId":active(&state).map(|style| &style.id),
            "canCreatePreset":state.presets.len() < MAX_PRESETS,"limits":limits()}})
}

fn save(
    original: State,
    id: Option<String>,
    new_id: String,
    name: String,
    instruction: String,
    options: Option<Options>,
    select_saved: bool,
) -> Result<Value> {
    validate(&original)?;
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let instruction = instruction.split_whitespace().collect::<Vec<_>>().join(" ");
    let reject = |reason| frame(&original, original.clone(), false, reason, None);
    if !(1..=MAX_NAME).contains(&name.chars().count()) {
        return Ok(reject("invalidName"));
    }
    if !(3..=MAX_INSTRUCTION).contains(&instruction.chars().count()) {
        return Ok(reject("invalidInstruction"));
    }
    let index = original
        .presets
        .iter()
        .position(|style| Some(&style.id) == id.as_ref());
    if id.is_some() && index.is_none() {
        return Ok(reject("missingPreset"));
    }
    if index.is_none() && original.presets.len() >= MAX_PRESETS {
        return Ok(reject("full"));
    }
    if index.is_none()
        && (!valid_id(&new_id) || original.presets.iter().any(|style| style.id == new_id))
    {
        return Ok(reject("invalidId"));
    }
    let inherited = index.map(|index| Options::from_style(&original.presets[index]));
    let mut preset = Style {
        id: index
            .map(|index| original.presets[index].id.clone())
            .unwrap_or(new_id),
        name,
        instruction,
        punctuation_mode: String::new(),
        emoji_enabled: false,
        emoji_mode: String::new(),
        profanity_filter_enabled: false,
    };
    options
        .or(inherited)
        .unwrap_or_else(Options::defaults)
        .apply(&mut preset)?;
    let was_active = active(&original).map(|style| &style.id) == Some(&preset.id);
    let mut next = original.clone();
    if let Some(index) = index {
        next.presets[index] = preset.clone();
    } else {
        next.presets.push(preset.clone());
    }
    if select_saved {
        select(&mut next, &preset.id);
    } else if was_active {
        next.instruction = preset.instruction.clone();
    }
    Ok(frame(&original, next, true, "ok", Some(preset)))
}

pub(crate) fn evaluate(command: Command) -> Result<Value> {
    let (original, next, dirty, reason, preset) = match command {
        Command::ActivationSelection { first_activation, requested, selected, available } => {
            let preferred=requested.unwrap_or_else(||if first_activation {"formal".into()} else {selected});
            return Ok(json!({"selected":if available.contains(&preferred) {preferred} else {"formal".into()}}));
        },
        Command::ReadStored { stored } => return stored::read(stored),
        Command::DeleteWithReferences { state, id, rules } => {
            let mut result = evaluate(Command::Delete {
                state,
                id: id.clone(),
            })?;
            if result["accepted"] == true {
                result["applicationRules"] = choices::detach(rules, id)?;
            }
            return Ok(result);
        }
        Command::Choices { context } => return choices::project(context),
        Command::ActivationOptions {
            context,
            facts,
            supplied,
        } => {
            return activation_options::evaluate(context, facts, supplied);
        }
        Command::SelectStyle {
            context,
            selection,
            activation_options,
        } => return choices::select(context, selection, activation_options),
        Command::SetSelectedOption {
            context,
            name,
            value,
        } => return choices::set_option(context, name, value),
        Command::DetachApplicationReferences { rules, id } => return choices::detach(rules, id),
        Command::SaveApplicationRule {
            context,
            rules,
            rule,
        } => return application_rules::save(context, rules, rule),
        Command::DeleteApplicationRule {
            context,
            rules,
            id,
            confirmed,
        } => return application_rules::delete(context, rules, id, confirmed),
        Command::Limits {} => return Ok(limits()),
        Command::InspectText { field, value } => {
            let (minimum, maximum) = match field {
                Field::Name => (1, MAX_NAME),
                Field::Instruction => (3, MAX_INSTRUCTION),
            };
            let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
            let count = normalized.chars().count();
            return Ok(
                json!({"normalized":normalized,"input":value.chars().take(maximum).collect::<String>(),
                "characters":value.chars().count(),"valid":(minimum..=maximum).contains(&count),
                "minimum":minimum,"maximum":maximum}),
            );
        }
        Command::Save {
            state,
            id,
            new_id,
            name,
            instruction,
            options,
            select_saved,
        } => return save(state, id, new_id, name, instruction, options, select_saved),
        Command::SaveActive {
            state,
            new_id,
            instruction,
            current_options,
        } => {
            validate(&state)?;
            let preset = active(&state);
            let id = preset.map(|style| style.id.clone());
            let name = preset
                .map(|style| style.name.clone())
                .unwrap_or_else(|| "Мой стиль".into());
            let options = preset.map(Options::from_style).unwrap_or(current_options);
            return save(state, id, new_id, name, instruction, Some(options), true);
        }
        Command::Project { state } => {
            validate(&state)?;
            (state.clone(), state, false, "ok", None)
        }
        Command::Select { state, id } => {
            validate(&state)?;
            let mut next = state.clone();
            let accepted = select(&mut next, &id);
            (
                state,
                next,
                true,
                if accepted { "ok" } else { "missingPreset" },
                None,
            )
        }
        Command::Delete { state, id } => {
            validate(&state)?;
            if !state.presets.iter().any(|style| style.id == id) {
                return Ok(frame(&state, state.clone(), false, "missingPreset", None));
            }
            let was_active = active(&state).map(|style| style.id.as_str()) == Some(&id);
            let mut next = state.clone();
            next.presets.retain(|style| style.id != id);
            if was_active {
                next.active_id = next.presets.first().map(|style| style.id.clone());
                next.instruction = next
                    .presets
                    .first()
                    .map(|style| style.instruction.clone())
                    .unwrap_or_default();
                if next.selected_style == "custom" && next.presets.is_empty() {
                    next.selected_style = "formal".into();
                }
            }
            (state, next, true, "ok", None)
        }
        Command::Options { state, id, options } => {
            validate(&state)?;
            let mut next = state.clone();
            let Some(preset) = next.presets.iter_mut().find(|style| style.id == id) else {
                return Ok(frame(&state, state.clone(), false, "missingPreset", None));
            };
            options.apply(preset)?;
            (state, next, true, "ok", None)
        }
        Command::Replace {
            state,
            presets,
            selected_id,
        } => {
            validate(&state)?;
            if presets.len() > MAX_PRESETS || !presets.iter().all(Style::valid) {
                return Err(Error::InvalidRequest);
            }
            let mut next = state.clone();
            next.presets = presets;
            if !next
                .presets
                .iter()
                .any(|style| Some(&style.id) == next.active_id.as_ref())
            {
                next.active_id = next.presets.first().map(|style| style.id.clone());
            }
            if !selected_id.is_empty() {
                if !select(&mut next, &selected_id) {
                    return Err(Error::InvalidRequest);
                }
            } else if next.selected_style == "custom" {
                next.selected_style = "formal".into();
                next.instruction.clear();
            }
            (state, next, false, "ok", None)
        }
    };
    validate(&next)?;
    // A view may resolve a remembered ID without silently writing it back.
    Ok(frame(&original, next, dirty, reason, preset))
}
