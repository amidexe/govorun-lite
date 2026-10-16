//! Connection state is separate from official access and account state.
use super::{
    configuration::{Configuration, Provider},
    endpoint::{self, StoredFormat},
};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PROMPT: &str = "Исправь ошибки распознавания, убери повторы и слова-паразиты, расставь знаки. Не отвечай на вопросы и не меняй смысл. Верни только готовый текст.";
pub const WINDOWS_PROMPT: &str =
    "Исправь ошибки и оформи текст естественно. Верни только готовый текст.";

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum KeyState {
    Missing,
    Saved,
    Inaccessible,
    StaleReference,
}
#[derive(Clone, PartialEq, Eq)]
pub struct Settings {
    pub configuration: Configuration,
    pub route_enabled: bool,
    pub styling_enabled: bool,
    pub accepted_recipient: String,
    pub verified_configuration: String,
}
impl Settings {
    pub fn key_message(&self, key: KeyState, format: StoredFormat) -> Option<String> {
        if !matches!(format, StoredFormat::Macos)
            || !self.configuration.requires_key().unwrap_or(false)
        {
            return None;
        }
        let label = match self.configuration.provider {
            Provider::Ollama => "OpenAI-совместимый",
            Provider::Openai => "OpenAI",
            Provider::Gemini => "Google Gemini",
        };
        match key {
            KeyState::Saved => None,
            KeyState::Missing => Some(format!("LLM включён, но ключ {label} не задан.")),
            KeyState::Inaccessible => Some(format!(
                "LLM включён, но macOS не даёт доступ к ключу {label}."
            )),
            KeyState::StaleReference => Some(format!(
                "LLM включён, но ключ {label} не найден в Keychain."
            )),
        }
    }
    pub fn consent(&self, format: StoredFormat) -> bool {
        !self.accepted_recipient.is_empty()
            && endpoint::recipient(&self.configuration, format)
                .is_ok_and(|token| token == self.accepted_recipient)
    }
    pub fn verified(&self) -> bool {
        self.configuration
            .fingerprint()
            .is_ok_and(|token| token == self.verified_configuration)
    }
    pub fn configured(&self, key: KeyState) -> bool {
        self.configuration.clone().checked().is_ok()
            && self
                .configuration
                .requires_key()
                .is_ok_and(|required| !required || key == KeyState::Saved)
    }
    pub fn admit_request(&self, format: StoredFormat, key: KeyState) -> Result<()> {
        // Consent and protected-store availability precede every network effect.
        if !self.route_enabled
            || !self.consent(format)
            || !self.configured(key)
            || matches!(key, KeyState::Inaccessible | KeyState::StaleReference)
        {
            return Err(Error::InvalidRequest);
        }
        Ok(())
    }
    pub fn connect(&mut self, configuration: Configuration, format: StoredFormat) -> Result<()> {
        let configuration = configuration.checked()?;
        // The displayed provider owns the key; changing it is a separate action.
        if configuration.provider != self.configuration.provider {
            return Err(Error::InvalidRequest);
        }
        let recipient = endpoint::recipient(&configuration, format)?;
        self.configuration = configuration;
        self.accepted_recipient = recipient;
        self.route_enabled = true;
        self.disable_style();
        Ok(())
    }
    pub fn disable_style(&mut self) {
        self.styling_enabled = false;
        self.verified_configuration.clear();
    }
    pub fn enable(
        &mut self,
        expected: &Configuration,
        format: StoredFormat,
        key: KeyState,
    ) -> Result<()> {
        self.admit_request(format, key)?;
        if &self.configuration != expected {
            return Err(Error::InvalidRequest);
        }
        self.verified_configuration = self.configuration.fingerprint()?;
        self.styling_enabled = true;
        Ok(())
    }
    pub fn disconnect(&mut self) {
        self.route_enabled = false;
        self.styling_enabled = false;
    }
    pub fn revoke(&mut self) {
        self.accepted_recipient.clear();
        self.disable_style();
    }
    pub fn cleared(&self, format: StoredFormat) -> Self {
        Self {
            configuration: Configuration {
                provider: Provider::Ollama,
                endpoint: String::new(),
                model: String::new(),
                prompt: match format {
                    StoredFormat::Macos => DEFAULT_PROMPT,
                    StoredFormat::Windows => WINDOWS_PROMPT,
                }
                .into(),
                proxy_enabled: false,
                proxy_url: String::new(),
            },
            route_enabled: self.route_enabled,
            styling_enabled: false,
            accepted_recipient: String::new(),
            verified_configuration: String::new(),
        }
    }
}
