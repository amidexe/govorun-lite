//! Current atomic account file layout. Only the credentials belong to the sign-in:
//! the retired `vault` field (ready passes live in their own pool since 2.1.x) and any
//! field another version adds are kept exactly as stored and never decide whether the
//! sign-in can be read (owner, 23 Oct 2026: one part never disables another).
use govorun_core::style_pass_vault::{format::Format, Envelope};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccountEnvelope {
    pub version: u32,
    pub credentials: Option<Credentials>,
    /// Retired pass journal of older versions, opaque.
    pub vault: Option<Value>,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}
impl Default for AccountEnvelope {
    fn default() -> Self {
        Self {
            version: 1,
            credentials: None,
            vault: None,
            other: Map::new(),
        }
    }
}
impl AccountEnvelope {
    pub fn decode(raw: &str) -> Result<Self, ()> {
        if raw.len() > 9 * 1_048_576 {
            return Err(());
        }
        let value: Value = serde_json::from_str(raw).map_err(|_| ())?;
        let fields = value.as_object().ok_or(())?;
        // A different layout version is a different format; its credentials are not ours.
        if fields.get("version") != Some(&Value::from(1)) {
            return Err(());
        }
        let mut envelope = Self {
            version: 1,
            credentials: None,
            vault: fields.get("vault").filter(|vault| !vault.is_null()).cloned(),
            other: fields
                .iter()
                .filter(|(key, _)| !matches!(key.as_str(), "version" | "credentials" | "vault"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        };
        envelope.credentials = match fields.get("credentials") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let credentials: Credentials = serde_json::from_value(value.clone()).map_err(|_| ())?;
                if !usable(&credentials) {
                    return Err(());
                }
                Some(credentials)
            }
        };
        Ok(envelope)
    }
    /// Ready passes stay refused while an older journal here cannot be read: the pass
    /// rules are unchanged, only the sign-in no longer depends on it.
    pub fn vault_supported(&self) -> bool {
        match &self.vault {
            None => true,
            Some(Value::String(raw)) => Envelope::decode(raw, Format::LedgerV5).is_ok(),
            Some(_) => false,
        }
    }
    pub fn encoded(&self) -> Result<String, ()> {
        if self.version != 1 || self.credentials.as_ref().is_some_and(|c| !usable(c)) {
            return Err(());
        }
        let raw = serde_json::to_string(self).map_err(|_| ())?;
        if raw.len() > 9 * 1_048_576 {
            return Err(());
        }
        Ok(raw)
    }
}
fn usable(credentials: &Credentials) -> bool {
    !credentials.access_token.is_empty()
        && !credentials.refresh_token.is_empty()
        && credentials.access_token.len() <= 65_536
        && credentials.refresh_token.len() <= 65_536
}
