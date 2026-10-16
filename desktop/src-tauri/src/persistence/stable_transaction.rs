//! Restore an existing interrupted Windows export before opening current stores.
//! No new export or marker can be created; retained solely to protect old user data.
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
}
pub type Documents = BTreeMap<String, Option<String>>;
const MARKER: &str = "stable-return-pending.json";
const ALLOWED: [&str; 7] = [
    "stats.json",
    "sync-state.json",
    "dictionary.json",
    "dictionary-return-v1.json",
    "statistics-journal-v1.json",
    "content-journal.json",
    "preference-journal.json",
];
pub trait Disk {
    fn read(&self, name: &str) -> Result<Option<String>, Error>;
    fn write(&self, name: &str, value: Option<&str>) -> Result<(), Error>;
    fn check(&self, name: Option<&str>) -> Result<(), Error>;
}
pub struct Files(pub PathBuf);
fn linked(path: &Path) -> Result<(), Error> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(Error::Unavailable),
    };
    if metadata.file_type().is_symlink() {
        return Err(Error::Unavailable);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(Error::Unavailable);
        }
    }
    Ok(())
}
impl Disk for Files {
    fn read(&self, name: &str) -> Result<Option<String>, Error> {
        let file = match std::fs::File::open(self.0.join(name)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::Unavailable),
        };
        let max = if name == MARKER {
            64 * 1_048_576
        } else {
            8 * 1_048_576
        };
        if file.metadata().map_err(|_| Error::Unavailable)?.len() > max {
            return Err(Error::Invalid);
        }
        let mut bytes = Vec::new();
        file.take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Unavailable)?;
        if bytes.len() > max as usize {
            return Err(Error::Invalid);
        }
        let raw = String::from_utf8(bytes).map_err(|_| Error::Invalid)?;
        Ok(Some(raw.trim_start_matches('\u{feff}').into()))
    }
    fn write(&self, name: &str, value: Option<&str>) -> Result<(), Error> {
        let path = self.0.join(name);
        self.check(Some(name))?;
        let Some(raw) = value else {
            return match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(Error::Unavailable),
            };
        };
        let temp = self.0.join(format!("{name}.return-tmp"));
        linked(&temp)?;
        let result = (|| {
            let mut file = std::fs::File::create(&temp).map_err(|_| Error::Unavailable)?;
            file.write_all(raw.as_bytes())
                .map_err(|_| Error::Unavailable)?;
            file.sync_all().map_err(|_| Error::Unavailable)?;
            drop(file);
            std::fs::rename(&temp, &path).map_err(|_| Error::Unavailable)
        })();
        // Match the old transaction's finally-cleanup; never unlink the primary on failure.
        let cleanup = match std::fs::remove_file(&temp) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Error::Unavailable),
        };
        result.and(cleanup)
    }
    fn check(&self, name: Option<&str>) -> Result<(), Error> {
        linked(&self.0)?;
        if let Some(name) = name {
            linked(&self.0.join(name))?;
        }
        Ok(())
    }
}
fn validate(disk: &impl Disk, documents: &Documents) -> Result<(), Error> {
    if documents.is_empty() || documents.keys().any(|key| !ALLOWED.contains(&key.as_str())) {
        return Err(Error::Invalid);
    }
    disk.check(None)?;
    for key in documents.keys() {
        disk.check(Some(key))?;
    }
    Ok(())
}
fn apply(disk: &impl Disk, documents: &Documents) -> Result<(), Error> {
    for (key, value) in documents {
        disk.write(key, value.as_deref())?;
    }
    Ok(())
}
pub fn recover(disk: &impl Disk) -> Result<(), Error> {
    disk.check(Some(MARKER))?;
    let Some(raw) = disk.read(MARKER)? else {
        return Ok(());
    };
    let before: Documents = serde_json::from_str(&raw).map_err(|_| Error::Invalid)?;
    validate(disk, &before)?;
    apply(disk, &before)?;
    disk.write(MARKER, None)
}
/// A transient rollback failure must not admit new writers: the next recovery
/// would otherwise overwrite work recorded after the interrupted restoration.
pub fn restore_before_open(disk: &impl Disk) -> Result<(), Error> {
    match recover(disk) {
        Ok(()) => Ok(()),
        Err(Error::Invalid) => disk.write(MARKER, None),
        Err(error) => Err(error),
    }
}
