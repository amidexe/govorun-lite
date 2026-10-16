//! Current atomic account file layout; vault validation is the common codec.
use govorun_core::style_pass_vault::{format::Format, Envelope};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccountEnvelope {
    pub version: u32,
    pub credentials: Option<Credentials>,
    pub vault: Option<String>,
}
impl Default for AccountEnvelope {
    fn default() -> Self {
        Self {
            version: 1,
            credentials: None,
            vault: None,
        }
    }
}
impl AccountEnvelope {
    pub fn decode(raw: &str) -> Result<Self, ()> {
        if raw.len() > 9 * 1_048_576 {
            return Err(());
        }
        let value: Value = serde_json::from_str(raw).map_err(|_| ())?;
        if !value.as_object().is_some_and(|v| {
            v.len() == 3
                && ["version", "credentials", "vault"]
                    .iter()
                    .all(|k| v.contains_key(*k))
        }) {
            return Err(());
        }
        let envelope: Self = serde_json::from_value(value).map_err(|_| ())?;
        envelope.validate()?;
        Ok(envelope)
    }
    pub fn encoded(&self) -> Result<String, ()> {
        self.validate()?;
        let raw = serde_json::to_string(self).map_err(|_| ())?;
        if raw.len() > 9 * 1_048_576 {
            return Err(());
        }
        Ok(raw)
    }
    fn validate(&self) -> Result<(), ()> {
        if self.version != 1 {
            return Err(());
        }
        if self.credentials.as_ref().is_some_and(|c| {
            c.access_token.is_empty()
                || c.refresh_token.is_empty()
                || c.access_token.len() > 65_536
                || c.refresh_token.len() > 65_536
        }) {
            return Err(());
        }
        if let Some(raw) = &self.vault {
            Envelope::decode(raw, Format::LedgerV5).map_err(|_| ())?;
        }
        Ok(())
    }
}
