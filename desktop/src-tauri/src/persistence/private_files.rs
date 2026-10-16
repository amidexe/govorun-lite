//! macOS secrets: private files, durable atomic replacement, no SDK prompts.
use super::secure_files::revision;
use std::{io::{Read, Write}, path::{Path, PathBuf}, sync::Mutex};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
const LIMIT: usize = 9 * 1_048_576;
#[cfg(target_os = "macos")]
#[path = "private_files/backup.rs"]
mod backup;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error { Invalid, Unavailable }
pub struct Store { root: PathBuf, gate: Mutex<()> }
impl Store {
    pub fn new(root: PathBuf) -> Self { Self { root, gate: Mutex::new(()) } }
    fn path(&self, service: &str, key: &str) -> Result<PathBuf, Error> {
        if service.is_empty() || key.is_empty() || service.len() > 256 || key.len() > 256 {
            return Err(Error::Invalid);
        }
        std::fs::create_dir_all(&self.root).map_err(|_| Error::Unavailable)?;
        let metadata = std::fs::symlink_metadata(&self.root).map_err(|_| Error::Unavailable)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() { return Err(Error::Unavailable); }
        #[cfg(unix)]
        std::fs::set_permissions(&self.root, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| Error::Unavailable)?;
        #[cfg(target_os = "macos")]
        backup::exclude(&self.root, true)?;
        Ok(self.root.join(format!("{}.dat", revision(&(service.to_owned() + "\0" + key)))))
    }
    fn read_path(path: &Path) -> Result<Option<String>, Error> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Unavailable),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() { return Err(Error::Unavailable); }
        if metadata.len() > LIMIT as u64 { return Err(Error::Invalid); }
        #[cfg(unix)]
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| Error::Unavailable)?;
        #[cfg(target_os = "macos")]
        backup::exclude(path, false)?;
        let mut bytes = Vec::new();
        std::fs::File::open(path).map_err(|_| Error::Unavailable)?
            .take(LIMIT as u64 + 1).read_to_end(&mut bytes).map_err(|_| Error::Unavailable)?;
        if bytes.len() > LIMIT { return Err(Error::Invalid); }
        String::from_utf8(bytes).map(Some).map_err(|_| Error::Invalid)
    }
    pub fn read(&self, service: &str, key: &str) -> Result<Option<String>, Error> {
        let _guard = self.gate.lock().map_err(|_| Error::Unavailable)?;
        Self::read_path(&self.path(service, key)?)
    }
    pub fn write(&self, service: &str, key: &str, raw: &str) -> Result<(), Error> {
        let _guard = self.gate.lock().map_err(|_| Error::Unavailable)?;
        if raw.len() > LIMIT { return Err(Error::Invalid); }
        let path = self.path(service, key)?;
        // Refuse a foreign link before replacement; never follow a fixed .tmp.
        Self::read_path(&path)?;
        let temporary = self.root.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)] options.mode(0o600);
            let mut file = options.open(&temporary).map_err(|_| Error::Unavailable)?;
            #[cfg(target_os = "macos")]
            backup::exclude(&temporary, false)?;
            file.write_all(raw.as_bytes()).map_err(|_| Error::Unavailable)?;
            file.sync_all().map_err(|_| Error::Unavailable)?;
            drop(file);
            std::fs::rename(&temporary, &path).map_err(|_| Error::Unavailable)?;
            #[cfg(target_os = "macos")]
            backup::exclude(&path, false)?;
            std::fs::File::open(&self.root).and_then(|file| file.sync_all()).map_err(|_| Error::Unavailable)?;
            if Self::read_path(&path)?.as_deref() != Some(raw) { return Err(Error::Unavailable); }
            Ok(())
        })();
        let _ = std::fs::remove_file(temporary);
        result
    }
    pub fn delete(&self, service: &str, key: &str) -> Result<(), Error> {
        let _guard = self.gate.lock().map_err(|_| Error::Unavailable)?;
        let path = self.path(service, key)?;
        Self::read_path(&path)?;
        match std::fs::remove_file(path) {
            Ok(()) => std::fs::File::open(&self.root).and_then(|file| file.sync_all()).map_err(|_| Error::Unavailable),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Error::Unavailable),
        }
    }
}
