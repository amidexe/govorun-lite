use serde::{Deserialize, Serialize};

pub(crate) const MAX_PRESETS: usize = 4;
pub(crate) const MAX_NAME: usize = 24;
pub(crate) const MAX_INSTRUCTION: usize = 200;

/// Current portable preset shape, shared by local edits and content synchronization.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Style {
    pub id: String,
    pub name: String,
    pub instruction: String,
    pub punctuation_mode: String,
    pub emoji_enabled: bool,
    pub emoji_mode: String,
    pub profanity_filter_enabled: bool,
}

impl Style {
    /// The editor's spacing rule. Older editors counted length without edge
    /// spaces, so an old style can exceed a limit only by its spacing.
    pub fn normalize(&mut self) {
        self.name = words(&self.name);
        self.instruction = words(&self.instruction);
    }
    pub fn valid(&self) -> bool {
        self.admissible_stored()
            && (1..=MAX_NAME).contains(&self.name.chars().count())
            && (3..=MAX_INSTRUCTION).contains(&self.instruction.chars().count())
    }

    /// Existing local text can exceed a newer edit limit. Admit bounded stored
    /// data without truncating it; new edits and synchronized documents stay strict.
    pub fn admissible_stored(&self) -> bool {
        valid_id(&self.id) && self.name.len() <= 8 * 1024 && self.instruction.len() <= 64 * 1024
            && matches!(self.punctuation_mode.as_str(), "less" | "normal")
            && matches!(self.emoji_mode.as_str(), "off" | "restrained" | "lively" | "bright")
            && self.emoji_enabled == (self.emoji_mode != "off")
    }
}

/// A stored preset this version can use. Others stay in the file and are skipped.
pub(crate) fn usable_stored(value: &serde_json::Value) -> bool {
    serde_json::from_value::<Style>(value.clone()).is_ok_and(|style| style.admissible_stored())
}

pub(crate) fn words(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn valid_id(value: &str) -> bool {
    (1..=80).contains(&value.len())
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
