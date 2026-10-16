//! Codecs for the current physical stores. Unrelated fields are retained.
use super::{
    configuration::{Configuration, Provider},
    endpoint::StoredFormat,
    settings::{Settings, DEFAULT_PROMPT, WINDOWS_PROMPT},
};
use crate::{StoreError, StoreResult};
use serde_json::{json, Map, Value};

pub struct Stored {
    pub format: StoredFormat,
    original: Value,
    values: Map<String, Value>,
    pub settings: Settings,
}
impl Stored {
    pub fn decode(value: Value) -> StoreResult<Self> {
        let (format, values) = match value["source"].as_str() {
            Some("macos") => (
                StoredFormat::Macos,
                value["values"]
                    .as_object()
                    .cloned()
                    .ok_or(StoreError::Invalid)?,
            ),
            // The physical Windows port returns null only when settings.json
            // does not exist. A fresh installation has the same defaults as {}.
            Some("windows") if value.get("values") == Some(&Value::Null) => {
                (StoredFormat::Windows, Map::new())
            }
            Some("windows") => {
                let raw = value["values"].as_str().ok_or(StoreError::Invalid)?;
                let parsed: Value = serde_json::from_str(raw).map_err(|_| StoreError::Invalid)?;
                (
                    StoredFormat::Windows,
                    parsed.as_object().cloned().ok_or(StoreError::Invalid)?,
                )
            }
            _ => return Err(StoreError::Invalid),
        };
        let settings = match format {
            StoredFormat::Macos => mac_settings(&values)?,
            StoredFormat::Windows => windows_settings(&values)?,
        };
        Ok(Self {
            format,
            original: value,
            values,
            settings,
        })
    }
    pub fn select_provider(&mut self, provider: Provider) -> StoreResult<()> {
        if self.settings.configuration.provider == provider {
            return Ok(());
        }
        self.update_values()?;
        let (endpoint, model) = provider.defaults();
        match self.format {
            StoredFormat::Macos => {
                self.values
                    .insert("llmProvider".into(), json!(provider.name()));
                self.settings.configuration.endpoint = text(
                    &self.values,
                    &format!("llmServerURL_{}", provider.name()),
                    endpoint,
                )?;
                self.settings.configuration.model = text(
                    &self.values,
                    &format!("llmModel_{}", provider.name()),
                    model,
                )?;
            }
            StoredFormat::Windows => {
                let current = &self.settings.configuration;
                let memory = self
                    .values
                    .entry("customServerProviders")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or(StoreError::Invalid)?;
                memory.insert(
                    current.provider.name().into(),
                    json!({"endpoint":current.endpoint,"model":current.model}),
                );
                let selected = memory
                    .get(provider.name())
                    .cloned()
                    .unwrap_or_else(|| json!({"endpoint":endpoint,"model":model}));
                self.settings.configuration.endpoint = selected["endpoint"]
                    .as_str()
                    .ok_or(StoreError::Invalid)?
                    .into();
                self.settings.configuration.model = selected["model"]
                    .as_str()
                    .ok_or(StoreError::Invalid)?
                    .into();
            }
        }
        self.settings.configuration.provider = provider;
        self.settings.disable_style();
        Ok(())
    }
    pub fn clear(&mut self) {
        self.settings = self.settings.cleared(self.format);
        match self.format {
            StoredFormat::Macos => {
                self.values.clear();
                // Only self-hosted keys enter this physical document.
            }
            StoredFormat::Windows => {
                self.values
                    .insert("customServerProviders".into(), json!({}));
                self.values.insert("customServerStyles".into(), json!([]));
            }
        }
    }
    pub fn encode(mut self) -> StoreResult<Value> {
        self.update_values()?;
        match self.format {
            StoredFormat::Macos => self.original["values"] = Value::Object(self.values),
            StoredFormat::Windows => {
                self.original["values"] =
                    json!(serde_json::to_string(&self.values).map_err(|_| StoreError::Invalid)?)
            }
        }
        Ok(self.original)
    }
    fn update_values(&mut self) -> StoreResult<()> {
        let settings = &self.settings;
        let config = &settings.configuration;
        let fields = match self.format {
            StoredFormat::Macos => vec![
                ("llmProvider".into(), json!(config.provider.name())),
                (
                    format!("llmServerURL_{}", config.provider.name()),
                    json!(config.endpoint),
                ),
                (
                    format!("llmModel_{}", config.provider.name()),
                    json!(config.model),
                ),
                ("llmSystemPrompt".into(), json!(config.prompt)),
                ("llmProxyEnabled".into(), json!(config.proxy_enabled)),
                ("llmProxyURL".into(), json!(config.proxy_url)),
                ("llmEnabled".into(), json!(settings.route_enabled)),
                (
                    "llmStylizationEnabledV1".into(),
                    json!(settings.styling_enabled),
                ),
                (
                    "llmAcceptedRecipientV1".into(),
                    json!(settings.accepted_recipient),
                ),
                (
                    "llmVerifiedConfigurationV1".into(),
                    json!(settings.verified_configuration),
                ),
            ],
            StoredFormat::Windows => vec![
                ("customServerProvider".into(), json!(config.provider.name())),
                ("customServerEndpoint".into(), json!(config.endpoint)),
                ("customServerModel".into(), json!(config.model)),
                ("customServerPrompt".into(), json!(config.prompt)),
                (
                    "customServerProxyEnabled".into(),
                    json!(config.proxy_enabled),
                ),
                ("customServerProxyUrl".into(), json!(config.proxy_url)),
                ("customServerEnabled".into(), json!(settings.route_enabled)),
                (
                    "customServerStylingEnabled".into(),
                    json!(settings.styling_enabled),
                ),
                (
                    "customServerAcceptedRecipient".into(),
                    json!(settings.accepted_recipient),
                ),
                (
                    "customServerVerifiedConfiguration".into(),
                    json!(settings.verified_configuration),
                ),
            ],
        };
        self.values.extend(fields);
        if settings.styling_enabled {
            self.values.insert(
                match self.format {
                    StoredFormat::Macos => "llmSelectedStyleV1",
                    StoredFormat::Windows => "customServerStyle",
                }
                .into(),
                json!("general"),
            );
        }
        Ok(())
    }
}
fn text(values: &Map<String, Value>, key: &str, default: &str) -> StoreResult<String> {
    match values.get(key) {
        None => Ok(default.into()),
        Some(value) => value.as_str().map(str::to_owned).ok_or(StoreError::Invalid),
    }
}
fn flag(values: &Map<String, Value>, key: &str) -> StoreResult<bool> {
    match values.get(key) {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or(StoreError::Invalid),
    }
}
fn provider(value: &str) -> StoreResult<Provider> {
    serde_json::from_value(json!(value)).map_err(|_| StoreError::Invalid)
}
fn mac_settings(v: &Map<String, Value>) -> StoreResult<Settings> {
    let legacy = if text(v, "llmEndpointType", "")? == "gemini" {
        "gemini"
    } else {
        "ollama"
    };
    let raw = text(v, "llmProvider", legacy)?;
    let provider = provider(&raw)?;
    let (endpoint, model) = provider.defaults();
    let mut prompt = text(v, "llmSystemPrompt", DEFAULT_PROMPT)?;
    let selected = text(v, "llmSelectedStyleV1", "general")?;
    if let Some(id) = selected.strip_prefix("custom:") {
        if let Some(raw) = v.get("llmCustomPresetsV1") {
            let presets: Value = serde_json::from_str(raw.as_str().ok_or(StoreError::Invalid)?)
                .map_err(|_| StoreError::Invalid)?;
            if let Some(preset) = presets
                .as_array()
                .ok_or(StoreError::Invalid)?
                .iter()
                .find(|value| value["id"] == id)
            {
                prompt = preset["instruction"]
                    .as_str()
                    .ok_or(StoreError::Invalid)?
                    .into();
            }
        }
    }
    Ok(Settings {
        configuration: Configuration {
            provider,
            endpoint: text(v, &format!("llmServerURL_{raw}"), endpoint)?,
            model: text(v, &format!("llmModel_{raw}"), model)?,
            prompt,
            proxy_enabled: flag(v, "llmProxyEnabled")?,
            proxy_url: text(v, "llmProxyURL", "")?,
        },
        route_enabled: flag(v, "llmEnabled")?,
        styling_enabled: flag(v, "llmStylizationEnabledV1")?,
        accepted_recipient: text(v, "llmAcceptedRecipientV1", "")?,
        verified_configuration: text(v, "llmVerifiedConfigurationV1", "")?,
    })
}
fn windows_settings(v: &Map<String, Value>) -> StoreResult<Settings> {
    Ok(Settings {
        configuration: Configuration {
            provider: provider(&text(v, "customServerProvider", "ollama")?)?,
            endpoint: text(v, "customServerEndpoint", "")?,
            model: text(v, "customServerModel", "")?,
            prompt: text(v, "customServerPrompt", WINDOWS_PROMPT)?,
            proxy_enabled: flag(v, "customServerProxyEnabled")?,
            proxy_url: text(v, "customServerProxyUrl", "")?,
        },
        route_enabled: flag(v, "customServerEnabled")?,
        styling_enabled: flag(v, "customServerStylingEnabled")?,
        accepted_recipient: text(v, "customServerAcceptedRecipient", "")?,
        verified_configuration: text(v, "customServerVerifiedConfiguration", "")?,
    })
}
