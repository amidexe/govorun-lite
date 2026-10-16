use crate::{Error, Future, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum AuthProvider { Yandex, Vk }

impl AuthProvider {
    fn name(self) -> &'static str { match self { Self::Yandex => "yandex", Self::Vk => "vk" } }
}

/// Sensitive material is carried only by the private OS storage adapter.
/// Deliberately no Debug/Display and never part of an account/UI view.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthAttempt {
    pub version: u32,
    pub provider: AuthProvider,
    pub verifier: String,
    pub state: String,
    pub handoff: Option<String>,
}

impl AuthAttempt {
    pub fn validate(&self) -> Result<()> {
        let safe = |s: &str| s.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'));
        if self.version != 1 || !(43..=128).contains(&self.verifier.len()) || !safe(&self.verifier) ||
            !(22..=128).contains(&self.state.len()) || !safe(&self.state) ||
            self.handoff.as_deref().is_some_and(|s| !bounded(s, 4096)) {
            return Err(Error::StorageInvalid);
        }
        Ok(())
    }

    pub(crate) fn create(provider: AuthProvider) -> Result<Self> {
        let random = |size| -> Result<String> {
            let mut bytes = vec![0; size];
            getrandom::fill(&mut bytes).map_err(|_| Error::StorageUnavailable)?;
            Ok(URL_SAFE_NO_PAD.encode(bytes))
        };
        Ok(Self { version: 1, provider, verifier: random(48)?, state: random(24)?, handoff: None })
    }
}

/// Save(None) must persist cancellation before deleting legacy attempt keys.
/// OS identities remain unchanged; loading can migrate the previous native keys.
pub trait AuthStorage: Send + Sync {
    fn load_attempt(&self) -> Future<'_, Result<Option<AuthAttempt>>>;
    fn save_attempt(&self, attempt: Option<AuthAttempt>, allow_interaction: bool) -> Future<'_, Result<()>>;
}

pub struct AuthConfig {
    origin: url::Url,
    platform: &'static str,
    scheme: &'static str,
    pub(crate) refresh_import: bool,
}

impl AuthConfig {
    pub fn new(origin: &str, platform: &str) -> Result<Self> {
        let origin = crate::configuration::accounts_origin(origin)?;
        let platform = match platform { "windows" => "windows", "macos" => "macos", "android" => "android", _ => return Err(Error::InvalidRequest) };
        let scheme = crate::configuration::callback_scheme(&origin)?;
        Ok(Self { origin, platform, scheme, refresh_import: false })
    }

    /// Trusted composition only; a command or file cannot enable credential import.
    pub fn with_refresh_import(mut self) -> Self {
        self.refresh_import = true;
        self
    }

    pub(crate) fn start_url(&self, attempt: &AuthAttempt) -> Result<String> {
        attempt.validate()?;
        let provider = attempt.provider.name();
        let mut url = self.origin.join(&format!("/v1/auth/{provider}/start")).map_err(|_| Error::InvalidRequest)?;
        url.query_pairs_mut().extend_pairs([
            ("platform", self.platform), ("return_to", &format!("{}://auth/{provider}", self.scheme)),
            ("state", &attempt.state), ("code_challenge", &URL_SAFE_NO_PAD.encode(Sha256::digest(attempt.verifier.as_bytes()))),
            ("code_challenge_method", "S256"), ("user_agreement_version", "1.0"),
        ]);
        Ok(url.into())
    }

    pub(crate) fn callback(&self, attempt: &AuthAttempt, callback: &str) -> Result<String> {
        attempt.validate()?;
        if callback.len() > 8192 { return Err(Error::InvalidRequest); }
        let url = url::Url::parse(callback).map_err(|_| Error::InvalidRequest)?;
        if url.scheme() != self.scheme || url.host_str() != Some("auth") ||
            url.path() != format!("/{}", attempt.provider.name()) || !url.username().is_empty() ||
            url.password().is_some() || url.port().is_some() || url.fragment().is_some() {
            return Err(Error::InvalidRequest);
        }
        let mut state = None;
        let mut handoff = None;
        for (key, value) in url.query_pairs() {
            let target = match key.as_ref() { "state" => &mut state, "handoff" => &mut handoff, _ => continue };
            if target.replace(value.into_owned()).is_some() { return Err(Error::InvalidRequest); }
        }
        let state = state.ok_or(Error::InvalidRequest)?;
        let handoff = handoff.filter(|s| bounded(s, 4096)).ok_or(Error::InvalidRequest)?;
        // Compare all bytes after the public length check; no prefix/early exit.
        if state.len() != attempt.state.len() || state.bytes().zip(attempt.state.bytes()).fold(0u8, |difference, (a, b)| difference | (a ^ b)) != 0 {
            return Err(Error::InvalidRequest);
        }
        Ok(handoff)
    }
}

pub struct AuthDevice { pub installation_id: String, pub name: String }

impl AuthDevice {
    pub(crate) fn exchange(&self, attempt: &AuthAttempt, replace: Option<&str>) -> Result<Value> {
        if !uuid(&self.installation_id) || !bounded(&self.name, 120) || replace.is_some_and(|s| !uuid(s)) {
            return Err(Error::InvalidRequest);
        }
        let handoff = attempt.handoff.as_deref().ok_or(Error::InvalidRequest)?;
        Ok(json!({"handoff": handoff, "codeVerifier": attempt.verifier,
            "installationId": self.installation_id, "deviceName": self.name, "replaceDeviceId": replace}))
    }
}

fn bounded(value: &str, max: usize) -> bool { !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control) }
fn uuid(value: &str) -> bool {
    value.len() == 36 && value.bytes().enumerate().all(|(i, c)|
        if [8, 13, 18, 23].contains(&i) { c == b'-' } else { c.is_ascii_hexdigit() })
}
