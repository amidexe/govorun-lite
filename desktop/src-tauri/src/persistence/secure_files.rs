//! Protected login session and plain private user files; migrate existing user data on read.
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAXIMUM: usize = 24 * 1_048_576;
static GATE: Mutex<()> = Mutex::new(());
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
    Conflict,
}
pub trait Cryptography: Send + Sync {
    fn protect(&self, raw: &[u8]) -> Result<Vec<u8>, Error>;
    fn unprotect(&self, raw: &[u8]) -> Result<Vec<u8>, Error>;
}
pub struct Store<C> {
    root: PathBuf,
    crypto: C,
    temporary: Mutex<std::collections::BTreeMap<String, String>>,
}
pub fn revision(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}
pub fn checkout_key(key: &str) -> bool {
    key.strip_prefix("account-purchase-v1-")
        .is_some_and(|suffix| {
            suffix.len() == 64
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}
fn valid_key(key: &str) -> bool {
    checkout_key(key)
        || !key.is_empty()
            && key.len() <= 64
            && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
fn read_file(path: &Path) -> Result<Option<String>, Error> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Unavailable),
    };
    if file.metadata().map_err(|_| Error::Unavailable)?.len() > MAXIMUM as u64 {
        return Err(Error::Invalid);
    }
    let mut bytes = Vec::new();
    file.take(MAXIMUM as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > MAXIMUM {
        return Err(Error::Invalid);
    }
    let raw = String::from_utf8(bytes).map_err(|_| Error::Invalid)?;
    Ok(Some(raw))
}
fn atomic_write(path: &Path, raw: &str) -> Result<(), Error> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|_| Error::Unavailable)?;
    file.write_all(raw.as_bytes())
        .map_err(|_| Error::Unavailable)?;
    file.sync_all().map_err(|_| Error::Unavailable)?;
    drop(file);
    std::fs::rename(tmp, path).map_err(|_| Error::Unavailable)
}
impl<C: Cryptography> Store<C> {
    pub fn new(root: PathBuf, crypto: C) -> Self {
        Self { root, crypto, temporary: Mutex::new(std::collections::BTreeMap::new()) }
    }
    fn path(&self, key: &str) -> Result<PathBuf, Error> {
        if !valid_key(key) {
            return Err(Error::Invalid);
        }
        Ok(self.root.join(format!("{}.dat", Self::current_key(key))))
    }
    fn read_path(&self, path: &Path) -> Result<Option<String>, Error> {
        let Some(raw) = read_file(path)? else {
            return Ok(None);
        };
        if !path.file_name().is_some_and(|name| name == "account-session-v2.dat") { return Ok(Some(raw)); }
        let bytes = STANDARD
            .decode(
                raw.trim_start_matches('\u{feff}').chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>(),
            )
            .map_err(|_| Error::Invalid)?;
        let bytes = self.crypto.unprotect(&bytes)?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| Error::Invalid)
    }
    fn write_path(&self, path: &Path, raw: &str) -> Result<(), Error> {
        if raw.len() > 9 * 1_048_576 {
            return Err(Error::Invalid);
        }
        std::fs::create_dir_all(path.parent().ok_or(Error::Invalid)?)
            .map_err(|_| Error::Unavailable)?;
        let encrypted = if !path.file_name().is_some_and(|name| name == "account-session-v2.dat") { raw.to_string() }
            else { STANDARD.encode(self.crypto.protect(raw.as_bytes())?) };
        atomic_write(path, &encrypted)?;
        if self.read_path(path)?.as_deref() != Some(raw) {
            return Err(Error::Unavailable);
        }
        Ok(())
    }
    fn user_key(key: &str) -> bool {
        checkout_key(key) || matches!(key, "installation-id" | "custom-server-api-key" | "custom-server-openai-key" | "custom-server-gemini-key")
    }
    fn current_key(key: &str) -> String {
        if Self::user_key(key) { format!("{key}-v2") } else { key.into() }
    }
    fn read_current(&self, key: &str) -> Result<Option<String>, Error> {
        let current = self.path(key)?;
        if Self::user_key(key) {
            if let Some(raw) = self.read_path(&current)? { return Ok((!raw.is_empty()).then_some(raw)); }
            if let Some(raw) = read_file(&self.root.join(format!("{key}.dat")))? {
                let bytes = STANDARD.decode(raw.trim_start_matches('\u{feff}').trim()).map_err(|_| Error::Invalid)?;
                let decoded = String::from_utf8(self.crypto.unprotect(&bytes)?).map_err(|_| Error::Invalid)?;
                self.write_path(&current, &decoded)?;
                return Ok(Some(decoded));
            }
            return Ok(None);
        }
        self.read_path(&current)
    }
    pub fn read(&self, key: &str) -> Result<Option<String>, Error> {
        if key == "account-auth-attempt-v1" { return Ok(self.temporary.lock().map_err(|_|Error::Unavailable)?.get(key).cloned()); }
        let _guard = GATE.lock().map_err(|_| Error::Unavailable)?;
        self.read_current(key)
    }
    pub fn write(&self, key: &str, raw: &str) -> Result<(), Error> {
        if key == "account-auth-attempt-v1" { self.temporary.lock().map_err(|_|Error::Unavailable)?.insert(key.into(),raw.into()); return Ok(()); }
        let _guard = GATE.lock().map_err(|_| Error::Unavailable)?;
        self.write_path(&self.path(key)?, raw)
    }
    pub fn delete(&self, key: &str) -> Result<(), Error> {
        if key == "account-auth-attempt-v1" { self.temporary.lock().map_err(|_|Error::Unavailable)?.remove(key); return Ok(()); }
        let _guard = GATE.lock().map_err(|_| Error::Unavailable)?;
        if Self::user_key(key) {
            self.write_path(&self.path(key)?, "")?;
        }
        match std::fs::remove_file(if Self::user_key(key) { self.root.join(format!("{key}.dat")) } else { self.path(key)? }) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Error::Unavailable),
        }
    }
    pub fn compare_write(
        &self,
        key: &str,
        expected: Option<&str>,
        raw: &str,
    ) -> Result<String, Error> {
        let _guard = GATE.lock().map_err(|_| Error::Unavailable)?;
        let path = self.path(key)?;
        let current = self.read_current(key)?;
        if current.as_deref().map(revision).as_deref() != expected {
            return Err(Error::Conflict);
        }
        self.write_path(&path, raw)?;
        Ok(revision(raw))
    }
    pub fn restart(&self, _: &str) -> Result<(), Error> {
        self.delete("account-session-v2")?;
        self.delete("ready-pass-pool-v1")?;
        self.temporary.lock().map_err(|_|Error::Unavailable)?.clear();
        Ok(())
    }
}
