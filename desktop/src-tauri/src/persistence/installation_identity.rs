//! Existing installation-ID storage effects. Unreadable IDs never become new devices.
use super::{
    defaults_documents::{Cell, Domain},
    secure_files::{Cryptography, Store},
};
use std::sync::Mutex;
use uuid::Uuid;
static GATE: Mutex<()> = Mutex::new(());
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
}
pub trait PreviousIdentity {
    fn read(&self) -> Result<Option<String>, Error>;
}
const PREFERRED: &str = "govorunInstallationIdentityV2";
fn mac_uuid(raw: &str) -> bool {
    raw.len() == 36
        && [8, 13, 18, 23]
            .iter()
            .all(|index| raw.as_bytes()[*index] == b'-')
        && Uuid::parse_str(raw).is_ok()
}
fn windows_uuid(raw: &str) -> Option<Uuid> {
    let raw = raw.trim();
    let inner = raw
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(raw);
    if let Ok(id) = Uuid::parse_str(inner) {
        return (!inner.starts_with("urn:")).then_some(id);
    }
    // Guid's historical X representation; never accept an arbitrary filtered string.
    let inner = raw.strip_prefix('{')?.strip_suffix('}')?;
    let (head, tail) = inner.split_once('{')?;
    let tail = tail.strip_suffix('}')?;
    let tokens: Vec<_> = head
        .trim_end()
        .strip_suffix(',')?
        .split(',')
        .chain(tail.split(','))
        .collect();
    if tokens.len() != 11 {
        return None;
    }
    let mut canonical = String::new();
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim();
        let hex = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))?;
        let width = match index {
            0 => 8,
            1 | 2 => 4,
            _ => 2,
        };
        if hex.is_empty() || hex.len() > width || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        canonical.extend(std::iter::repeat_n('0', width - hex.len()));
        canonical.push_str(hex);
    }
    Uuid::parse_str(&canonical).ok()
}
pub fn windows<C: Cryptography>(store: &Store<C>) -> Result<String, Error> {
    let _guard = GATE.lock().map_err(|_| Error::Unavailable)?;
    let map = |error| {
        if error == super::secure_files::Error::Invalid {
            Error::Invalid
        } else {
            Error::Unavailable
        }
    };
    if let Some(raw) = store.read("installation-id").map_err(map)? {
        return windows_uuid(&raw)
            .map(|id| id.hyphenated().to_string())
            .ok_or(Error::Invalid);
    }
    let generated = Uuid::new_v4().hyphenated().to_string();
    store.write("installation-id", &generated).map_err(map)?;
    Ok(generated)
}
pub fn macos(domain: &impl Domain, previous: &impl PreviousIdentity) -> Result<String, Error> {
    let _guard = GATE.try_lock().map_err(|_| Error::Unavailable)?;
    let values = domain.read().map_err(|_| Error::Unavailable)?;
    if let Some(value) = values.get(PREFERRED) {
        return match value {
            Cell::Scalar(serde_json::Value::String(raw)) if mac_uuid(raw) => Ok(raw.clone()),
            _ => Err(Error::Invalid),
        };
    }
    let value = match previous.read()? {
        Some(raw) if mac_uuid(&raw) => raw,
        Some(_) => return Err(Error::Invalid),
        None => Uuid::new_v4().hyphenated().to_string(),
    };
    domain.write([(PREFERRED.into(), Some(Cell::Scalar(serde_json::json!(value))))].into())
        .map_err(|_| Error::Unavailable)?;
    if !matches!(domain.read().map_err(|_|Error::Unavailable)?.get(PREFERRED),
        Some(Cell::Scalar(serde_json::Value::String(raw))) if raw==&value) { return Err(Error::Unavailable); }
    Ok(value)
}

#[cfg(target_os = "macos")]
pub struct SystemPreviousIdentity(pub String);
#[cfg(target_os = "macos")]
impl PreviousIdentity for SystemPreviousIdentity {
    fn read(&self) -> Result<Option<String>, Error> {
        super::macos_user_keychain::Keychain::read(
            &super::macos_profile::llm_service(&self.0), "govorunInstallationId", false, true,
        ).map_err(|_| Error::Unavailable)
    }
}
