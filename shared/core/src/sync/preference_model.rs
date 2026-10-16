use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Values {
    pub default_style: String,
    pub punctuation_mode: String,
    pub emoji_mode: String,
    pub profanity_filter_enabled: bool,
}
impl Values {
    pub fn validate(&self, local: bool) -> Result<()> {
        if !(matches!(self.default_style.as_str(), "formal" | "light" | "concise")
            || (local && self.custom()))
            || !matches!(self.punctuation_mode.as_str(), "less" | "normal")
            || !matches!(
                self.emoji_mode.as_str(),
                "off" | "restrained" | "lively" | "bright"
            )
        {
            return Err(Error::InvalidRequest);
        }
        Ok(())
    }
    pub fn custom(&self) -> bool {
        self.default_style == "custom"
    }
    pub fn portable(&self) -> Self {
        if self.custom() {
            Self {
                default_style: "formal".into(),
                punctuation_mode: "normal".into(),
                emoji_mode: "off".into(),
                profanity_filter_enabled: false,
            }
        } else {
            self.clone()
        }
    }
}

#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Changes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profanity_filter_enabled: Option<bool>,
}
impl Changes {
    pub fn all(v: &Values) -> Self {
        Self {
            default_style: Some(v.default_style.clone()),
            punctuation_mode: Some(v.punctuation_mode.clone()),
            emoji_mode: Some(v.emoji_mode.clone()),
            profanity_filter_enabled: Some(v.profanity_filter_enabled),
        }
    }
    pub fn between(base: &Values, local: &Values) -> Self {
        if local.custom() {
            return Self::default();
        }
        Self {
            default_style: (base.default_style != local.default_style)
                .then(|| local.default_style.clone()),
            punctuation_mode: (base.punctuation_mode != local.punctuation_mode)
                .then(|| local.punctuation_mode.clone()),
            emoji_mode: (base.emoji_mode != local.emoji_mode).then(|| local.emoji_mode.clone()),
            profanity_filter_enabled: (base.profanity_filter_enabled
                != local.profanity_filter_enabled)
                .then_some(local.profanity_filter_enabled),
        }
    }
    pub fn overlay(&mut self, newer: Self) {
        if newer.default_style.is_some() {
            self.default_style = newer.default_style;
        }
        if newer.punctuation_mode.is_some() {
            self.punctuation_mode = newer.punctuation_mode;
        }
        if newer.emoji_mode.is_some() {
            self.emoji_mode = newer.emoji_mode;
        }
        if newer.profanity_filter_enabled.is_some() {
            self.profanity_filter_enabled = newer.profanity_filter_enabled;
        }
    }
    pub fn acknowledge(&mut self, sent: &Self) {
        if sent.default_style.is_some() && sent.default_style == self.default_style {
            self.default_style = None;
        }
        if sent.punctuation_mode.is_some() && sent.punctuation_mode == self.punctuation_mode {
            self.punctuation_mode = None;
        }
        if sent.emoji_mode.is_some() && sent.emoji_mode == self.emoji_mode {
            self.emoji_mode = None;
        }
        if sent.profanity_filter_enabled.is_some()
            && sent.profanity_filter_enabled == self.profanity_filter_enabled
        {
            self.profanity_filter_enabled = None;
        }
    }
    pub fn apply(&self, base: &Values) -> Values {
        Values {
            default_style: self
                .default_style
                .clone()
                .unwrap_or_else(|| base.default_style.clone()),
            punctuation_mode: self
                .punctuation_mode
                .clone()
                .unwrap_or_else(|| base.punctuation_mode.clone()),
            emoji_mode: self
                .emoji_mode
                .clone()
                .unwrap_or_else(|| base.emoji_mode.clone()),
            profanity_filter_enabled: self
                .profanity_filter_enabled
                .unwrap_or(base.profanity_filter_enabled),
        }
    }
    pub fn empty(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<()> {
        self.apply(&Values {
            default_style: "formal".into(),
            punctuation_mode: "normal".into(),
            emoji_mode: "off".into(),
            profanity_filter_enabled: false,
        })
        .validate(false)
    }
    pub fn wire(&self) -> Result<Value> {
        self.validate()?;
        let mut value = serde_json::to_value(self).map_err(|_| Error::InvalidRequest)?;
        if let Some(mode) = &self.punctuation_mode {
            value["reducedPunctuation"] = json!(mode == "less");
        }
        if let Some(mode) = &self.emoji_mode {
            value["emojiEnabled"] = json!(mode != "off");
        }
        Ok(value)
    }
}

/// Current UI commands, not a decoder for historical preference documents.
#[derive(Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InputChanges {
    default_style: Option<String>,
    reduced_punctuation: Option<bool>,
    punctuation_mode: Option<String>,
    emoji_enabled: Option<bool>,
    emoji_mode: Option<String>,
    profanity_filter_enabled: Option<bool>,
    short_phrase_threshold: Option<i64>,
}
impl InputChanges {
    pub fn normalize(self) -> Result<Changes> {
        if self
            .short_phrase_threshold
            .is_some_and(|v| !(0..=500).contains(&v))
        {
            return Err(Error::InvalidRequest);
        }
        let value = Changes {
            default_style: self.default_style.filter(|v| v != "custom"),
            punctuation_mode: self.punctuation_mode.or_else(|| {
                self.reduced_punctuation
                    .map(|v| if v { "less" } else { "normal" }.into())
            }),
            emoji_mode: self.emoji_mode.or_else(|| {
                self.emoji_enabled
                    .map(|v| if v { "bright" } else { "off" }.into())
            }),
            profanity_filter_enabled: self.profanity_filter_enabled,
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WireValues {
    default_style: String,
    reduced_punctuation: bool,
    punctuation_mode: String,
    emoji_enabled: bool,
    emoji_mode: String,
    profanity_filter_enabled: bool,
    short_phrase_threshold: i64,
}
impl WireValues {
    pub fn canonical(self) -> Result<Values> {
        if self.reduced_punctuation != (self.punctuation_mode == "less")
            || self.emoji_enabled != (self.emoji_mode != "off")
            || !(0..=500).contains(&self.short_phrase_threshold)
        {
            return Err(Error::InvalidRequest);
        }
        let value = Values {
            default_style: self.default_style,
            punctuation_mode: self.punctuation_mode,
            emoji_mode: self.emoji_mode,
            profanity_filter_enabled: self.profanity_filter_enabled,
        };
        value.validate(false)?;
        Ok(value)
    }
}
