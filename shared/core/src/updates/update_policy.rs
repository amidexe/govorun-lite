use serde::{Deserialize, Serialize};

pub const MAX_PREFERENCES_BYTES: usize = 132_096;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Channel { #[default] Stable, Test }

impl Channel {
    pub fn feed(self) -> &'static str { match self {
        Self::Stable => "https://govorun.club/updates/tauri/stable.json",
        Self::Test => "https://govorun.club/updates/tauri/test.json",
    } }
}

pub fn official_url(value: &str) -> Result<url::Url, &'static str> {
    if value.len() > 2048 { return Err("updates_invalid_feed"); }
    let url = url::Url::parse(value).map_err(|_| "updates_invalid_feed")?;
    if url.scheme() != "https" || url.host_str() != Some("govorun.club") || url.port().is_some()
        || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("updates_invalid_feed");
    }
    Ok(url)
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub dismissed: Option<String>, pub last_attempt: u64,
    #[serde(default)] pub channel: Option<Channel>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CheckReason { Automatic, Manual }
impl CheckReason {
    pub fn allowed(self, signed_in: bool) -> bool { self == Self::Manual || signed_in }
}

impl Preferences {
    pub fn select_channel(&mut self, channel: Channel) {
        self.channel = Some(channel);
        self.dismissed = None;
        self.last_attempt = 0;
    }
    pub fn due(&self, now: u64, reason: CheckReason) -> bool {
        reason == CheckReason::Manual || self.last_attempt == 0 || now < self.last_attempt
            || now.saturating_sub(self.last_attempt) >= 6 * 60 * 60
    }
}
