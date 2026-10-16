//! User-selected connection facts. No account, entitlement or official quota.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Ollama,
    Openai,
    Gemini,
}
impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Openai => "openai",
            Self::Gemini => "gemini",
        }
    }
    pub fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::Ollama => ("http://localhost:11434/v1", ""),
            Self::Openai => ("https://api.openai.com/v1", ""),
            Self::Gemini => (
                "https://generativelanguage.googleapis.com/v1beta",
                "gemini-2.5-flash-lite",
            ),
        }
    }
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Configuration {
    pub provider: Provider,
    pub endpoint: String,
    pub model: String,
    pub prompt: String,
    pub proxy_enabled: bool,
    #[serde(rename = "proxyURL")]
    pub proxy_url: String,
}
impl Configuration {
    pub fn checked(mut self) -> Result<Self> {
        if self.endpoint.len() > 8192
            || self.model.len() > 1000
            || self.prompt.len() > 262_144
            || self.proxy_url.len() > 8192
        {
            return Err(Error::InvalidRequest);
        }
        self.endpoint = self.endpoint.trim().into();
        self.model = self.model.trim().into();
        self.proxy_url = self.proxy_url.trim().into();
        super::endpoint::chat(&self.endpoint)?;
        if self.model.is_empty() || self.prompt.trim().chars().count() < 3 {
            return Err(Error::InvalidRequest);
        }
        if self.proxy_enabled {
            super::endpoint::proxy(&self.proxy_url)?;
        }
        Ok(self)
    }
    pub fn requires_key(&self) -> Result<bool> {
        let endpoint = super::endpoint::chat(&self.endpoint)?;
        Ok(self.provider != Provider::Ollama
            && !matches!(
                endpoint.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
            ))
    }
    pub fn fingerprint(&self) -> Result<String> {
        Ok(format!(
            "{}\n{}",
            super::endpoint::chat(&self.endpoint)?,
            self.model.trim()
        ))
    }
}
