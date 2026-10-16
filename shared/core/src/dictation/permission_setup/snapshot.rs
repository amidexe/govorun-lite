use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "MicrophoneDevice"))]
pub struct Device {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Microphone {
    Authorized,
    NotDetermined,
    Denied,
    Restricted,
    Unknown,
}
impl Microphone {
    /// Desktop Windows may omit a consent registry value; an explicit denial
    /// always blocks capture. macOS and Android require an authorized fact.
    pub fn can_record(&self, platform: &str) -> bool {
        matches!(self, Self::Authorized)
            || (platform == "windows" && matches!(self, Self::Unknown | Self::NotDetermined))
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Login {
    Enabled,
    Disabled,
    ApprovalRequired,
    Unavailable,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Duration {
    Local,
    Styles,
    Unlimited,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Acceleration {
    Automatic,
    Cpu,
    Gpu,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Marker {
    Off,
    Microphone,
    Text,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Permissions {
    pub microphone: Microphone,
    pub accessibility: bool,
}

fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}
fn nullable_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", schemars(rename = "DictationSnapshot"))]
pub struct Snapshot {
    pub microphones: Vec<Device>,
    #[serde(deserialize_with = "nullable_id")]
    pub microphone_id: Option<String>,
    pub launch_at_login: Login,
    pub play_sounds: bool,
    pub mute_audio: bool,
    pub max_minutes: u32,
    pub duration_mode: Duration,
    pub acceleration: Acceleration,
    pub voice_marker: Marker,
    pub marker_minimum: u64,
    pub permissions: Permissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<bool>"))]
    pub push_to_talk: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<bool>"))]
    pub keep_model_loaded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<bool>"))]
    pub always_on_microphone: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<Option<u32>>"))]
    pub input_channel: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "crate::features::dto::Omitted<u32>"))]
    pub input_channels: Option<u32>,
}
impl Snapshot {
    pub fn checked(&self) -> bool {
        let mut ids = BTreeSet::new();
        let text = |value: &str| !value.is_empty() && value.encode_utf16().count() <= 1024;
        let channels = self.push_to_talk.is_some()
            || self.keep_model_loaded.is_some()
            || self.input_channel.is_some()
            || self.input_channels.is_some();
        self.microphones.len() <= 256
            && self
                .microphones
                .iter()
                .all(|device| text(&device.id) && text(&device.name) && ids.insert(&device.id))
            && self.microphone_id.as_deref().is_none_or(text)
            && self.max_minutes <= 180
            && self.marker_minimum <= 9_007_199_254_740_991
            && (!channels
                || self.push_to_talk.is_some()
                    && self.keep_model_loaded.is_some()
                    && self
                        .input_channels
                        .is_some_and(|count| (1..=32).contains(&count))
                    && self.input_channel.as_ref().is_some_and(|channel| {
                        channel.is_null() || channel.as_u64().is_some_and(|channel| channel < 32)
                    }))
    }
}
