use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Host/private-storage transport only. Never serialize into a renderer event.
/// Deliberately does not implement Debug or Display.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub device_limit: u32,
}

impl Credentials {
    pub fn validate(&self) -> Result<()> {
        if [&self.access_token, &self.refresh_token].iter().any(|value|
            value.is_empty() || value.len() > 65_536 || value.bytes().any(|c| c.is_ascii_control())) {
            return Err(Error::StorageInvalid);
        }
        Ok(())
    }
}

/// Preserves server fields for native consumers without inventing client entitlements.
/// Only validated views, never this raw object, are emitted to the renderer.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "Value", into = "Value")]
pub struct Profile(Value);

impl TryFrom<Value> for Profile {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        if !value.is_object() || ["id", "currentDeviceId"].iter().any(|key|
            value.get(key).and_then(Value::as_str).is_none_or(|s| s.is_empty() || s.len() > 128)) {
            return Err(Error::InvalidResponse);
        }
        if !value.get("styleAccess").is_some_and(Value::is_object) ||
            !value.get("onlineConsent").is_some_and(Value::is_object) {
            return Err(Error::InvalidResponse);
        }
        Ok(Self(value))
    }
}

impl From<Profile> for Value { fn from(value: Profile) -> Self { value.0 } }

impl Profile {
    pub fn wire(&self) -> &Value { &self.0 }
    pub fn native_wire(&self, fresh: bool) -> Value {
        crate::project_free_access(self.0.clone(), fresh)
    }
    pub fn id(&self) -> &str { self.0["id"].as_str().expect("validated profile") }
    pub fn device_id(&self) -> &str { self.0["currentDeviceId"].as_str().expect("validated profile") }
    pub fn display_name(&self) -> &str { self.0["displayName"].as_str().unwrap_or("Аккаунт подключён") }
    pub fn needs_usage_activation(&self) -> bool {
        self.0["styleUsageAccountingAvailable"] == true && self.0["styleAccess"]["state"] == "active" &&
            self.0["styleAccess"]["usageBalance"].is_null()
    }
}
