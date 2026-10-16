//! Atomic private files for the current account and self-hosted keys.
use super::{Error, Keychain};
use super::super::private_files::{self, Store};
use std::{path::PathBuf, sync::Mutex};
pub trait UserKeys: Send + Sync {
    fn read(&self, service: &str, key: &str, allow: bool) -> Result<Option<String>, Error>;
}
#[cfg(target_os = "macos")]
pub struct SystemUserKeys;
#[cfg(target_os = "macos")]
impl UserKeys for SystemUserKeys {
    fn read(&self, service: &str, key: &str, allow: bool) -> Result<Option<String>, Error> {
        super::super::macos_user_keychain::Keychain::read(service, key, allow, true)
            .map_err(|e| match e {
                super::super::macos_user_keychain::Error::Denied => Error::Denied,
                super::super::macos_user_keychain::Error::Invalid => Error::Invalid,
                _ => Error::Unavailable,
            })
    }
}
pub struct Files { files: Store, user_keys: Option<Box<dyn UserKeys>>, temporary: Mutex<std::collections::BTreeMap<(String,String),String>>, writer: Mutex<()> }
impl Files {
    pub fn new(root: PathBuf) -> Self { Self { files: Store::new(root), user_keys: None, temporary: Mutex::new(std::collections::BTreeMap::new()), writer: Mutex::new(()) } }
    pub fn with_user_keys(root: PathBuf, source: impl UserKeys + 'static) -> Self {
        Self { user_keys: Some(Box::new(source)), ..Self::new(root) }
    }
}
fn error(value: private_files::Error) -> Error {
    match value { private_files::Error::Invalid => Error::Invalid,
        private_files::Error::Unavailable => Error::Unavailable }
}
impl Keychain for Files {
    fn new_session_envelope(&self, service: &str) -> Result<super::AccountEnvelope, Error> {
        let _guard = self.writer.lock().map_err(|_| Error::Unavailable)?;
        self.files.read(service, super::ENVELOPE).map_err(error)?
            .filter(|raw| !raw.is_empty()).as_deref()
            .map(super::AccountEnvelope::decode).transpose()
            .map(|value| value.unwrap_or_default()).map_err(|_| Error::Invalid)
    }
    fn read(&self, service: &str, key: &str, allow: bool, llm: bool) -> Result<Option<String>, Error> {
        let _guard = self.writer.lock().map_err(|_| Error::Unavailable)?;
        if key == super::AUTH { return Ok(self.temporary.lock().map_err(|_|Error::Unavailable)?.get(&(service.into(),key.into())).cloned()); }
        if let Some(raw) = self.files.read(service, key).map_err(error)? {
            return Ok((!raw.is_empty()).then_some(raw));
        }
        if llm && matches!(key, "llmApiKey_ollama" | "llmApiKey_openai" | "llmApiKey_gemini") {
            if let Some(source) = &self.user_keys {
                if let Some(raw) = source.read(service, key, allow)? {
                    self.files.write(service, key, &raw).map_err(error)?;
                    return Ok(Some(raw));
                }
            }
        }
        Ok(None)
    }
    fn write(&self, service: &str, key: &str, raw: &str, _: bool, _: bool) -> Result<(), Error> {
        let _guard = self.writer.lock().map_err(|_| Error::Unavailable)?;
        if key == super::AUTH { self.temporary.lock().map_err(|_|Error::Unavailable)?.insert((service.into(),key.into()),raw.into()); return Ok(()); }
        self.files.write(service, key, raw).map_err(error)
    }
    fn delete(&self, services: &[&str], key: &str, _: bool, _: bool) -> Result<(), Error> {
        let _guard = self.writer.lock().map_err(|_| Error::Unavailable)?;
        for service in services {
            if key == super::AUTH { self.temporary.lock().map_err(|_|Error::Unavailable)?.remove(&(service.to_string(),key.into())); }
            else { self.files.write(service, key, "").map_err(error)?; }
        }
        Ok(())
    }
}
