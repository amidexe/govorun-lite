//! Assignment policy and capture lifetime. Agents describe keys and execute OS effects.
pub mod settings;
pub mod workflow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Target { Dictation, Plain, Cancel }
impl Target {
    pub fn name(self) -> &'static str {
        match self { Self::Dictation => "dictation", Self::Plain => "plain", Self::Cancel => "cancel" }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gesture {
    pub key: u16,
    pub modifiers: u8,
    pub modifier_only: bool,
    pub control_side: u8,
    pub reserved_for_paste: bool,
    pub valid: bool,
    pub tokens: Vec<String>,
    pub raw: Value,
}
impl Gesture {
    pub fn checked(&self) -> bool {
        self.modifiers <= 15 && self.control_side <= 2 && !self.tokens.is_empty() && self.tokens.len() <= 8
            && self.tokens.iter().all(|token| !token.is_empty() && token.len() <= 64 && !token.chars().any(char::is_control))
            && serde_json::to_vec(&self.raw).is_ok_and(|raw| raw.len() <= 4096)
    }
}
pub fn overlaps(first: &Gesture, second: &Gesture) -> bool {
    let sides_match = first.control_side == 0 || second.control_side == 0 || first.control_side == second.control_side;
    let equal = first.key == second.key && first.modifiers == second.modifiers
        && first.modifier_only == second.modifier_only && sides_match;
    let prefix = |modifier: &Gesture, chord: &Gesture| modifier.modifier_only && !chord.modifier_only
        && modifier.modifiers != 0 && chord.modifiers & modifier.modifiers == modifier.modifiers
        && (modifier.modifiers & 1 == 0 || sides_match);
    equal || prefix(first, second) || prefix(second, first)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Facts {
    pub dictation: Gesture,
    pub plain: Option<Gesture>,
    pub cancel: Gesture,
    pub default_dictation: Gesture,
    pub default_cancel: Gesture,
    pub busy: bool,
    pub permission_ready: bool,
    pub input_mode: Option<String>,
}
impl Facts {
    pub fn checked(&self) -> bool {
        [&self.dictation, &self.cancel, &self.default_dictation, &self.default_cancel].into_iter().all(Gesture::checked)
            && self.plain.as_ref().is_none_or(Gesture::checked)
            && self.input_mode.as_ref().is_none_or(|mode| matches!(mode.as_str(), "hold" | "adaptive"))
    }
    pub fn normalized(mut self) -> Self {
        if !self.dictation.valid || self.dictation.reserved_for_paste { self.dictation = self.default_dictation.clone(); }
        if !self.cancel.valid || self.cancel.reserved_for_paste { self.cancel = self.default_cancel.clone(); }
        self
    }
    pub fn effective_plain(&self) -> Option<&Gesture> {
        self.plain.as_ref().filter(|plain| plain.valid && !plain.reserved_for_paste
            && !overlaps(plain, &self.dictation) && !overlaps(plain, &self.cancel))
    }
    pub fn conflicts(&self, target: Target, gesture: &Gesture) -> bool {
        [(Target::Dictation, Some(&self.dictation)), (Target::Plain, self.effective_plain()), (Target::Cancel, Some(&self.cancel))]
            .into_iter().any(|(other, value)| other != target && value.is_some_and(|value| overlaps(value, gesture)))
    }
    pub fn projection(&self) -> Value {
        let mut result = json!({"dictation":self.dictation.tokens,
            "plain":self.effective_plain().map(|g|g.tokens.clone()).unwrap_or_default(),"cancel":self.cancel.tokens,
            "canCapture":!self.busy && self.permission_ready});
        if let Some(mode) = &self.input_mode { result["inputMode"] = json!(mode); }
        result
    }
}
