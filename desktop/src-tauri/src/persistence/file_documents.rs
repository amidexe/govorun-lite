//! Opaque Windows dictionary/statistics/sync bytes at the existing locations.
//! This adapter never interprets counters, merges content, or removes old files.
#[path = "settings_document.rs"]
pub(super) mod settings_document;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

const MAXIMUM: usize = 8 * 1_048_576;
const DICTIONARY_MAXIMUM: usize = 2 * 1_048_576;
pub(super) static WRITER: Mutex<()> = Mutex::new(());
pub use govorun_document_store::Error;
pub trait Disk: Send + Sync {
    fn read(&self, name: &str, maximum: usize) -> Result<Option<String>, Error>;
    fn write(&self, name: &str, raw: &str) -> Result<(), Error>;
}
pub struct Files {
    root: PathBuf,
}
impl Files {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}
fn io(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::InvalidData {
        Error::Invalid
    } else {
        Error::Unavailable
    }
}
impl Disk for Files {
    fn read(&self, name: &str, maximum: usize) -> Result<Option<String>, Error> {
        let file = match std::fs::File::open(self.root.join(name)) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io(error)),
        };
        if file.metadata().map_err(io)?.len() > maximum as u64 {
            return Err(Error::Invalid);
        }
        let mut bytes = Vec::new();
        file.take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        if bytes.len() > maximum {
            return Err(Error::Invalid);
        }
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| Error::Invalid)
    }
    fn write(&self, name: &str, raw: &str) -> Result<(), Error> {
        atomic_write(&self.root.join(name), raw)
    }
}
fn atomic_write(path: &Path, raw: &str) -> Result<(), Error> {
    std::fs::create_dir_all(path.parent().ok_or(Error::Invalid)?).map_err(io)?;
    // The old adapter uses this same sibling temporary name and replacement.
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let mut file = std::fs::File::create(&temporary).map_err(io)?;
    file.write_all(raw.as_bytes()).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    std::fs::rename(temporary, path).map_err(io)
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|fields| {
        fields.len() == keys.len() && keys.iter().all(|key| fields.contains_key(*key))
    })
}
fn revision(value: &Value) -> Result<String, Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Invalid)?;
    if bytes.len() > MAXIMUM {
        return Err(Error::Invalid);
    }
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn available(value: Value) -> Result<Value, Error> {
    let revision = revision(&value)?;
    Ok(json!({"status":"available","value":value,"revision":revision}))
}
/// The caller holds WRITER for the complete stable-return transaction.
pub(super) fn observed(disk: &impl Disk, key: &str) -> Result<Value, Error> {
    available(read(disk, key)?)
}
fn journal(key: &str) -> Option<&'static str> {
    match key {
        "sync.content" => Some("content-journal.json"),
        "sync.preferences" => Some("preference-journal.json"),
        "sync.enrollment" => Some("account-data-sync.json"),
        _ => None,
    }
}
fn read(disk: &impl Disk, key: &str) -> Result<Value, Error> {
    if settings_document::handles(key) {
        return Ok(json!({"source":"windows","values":disk.read("settings.json", MAXIMUM)?}));
    }
    if let Some(file) = journal(key) {
        return Ok(json!({"source":"windows","stored":disk.read(file,MAXIMUM)?}));
    }
    match key {
        "dictionary.document" => Ok(
            json!({"layout":"combined","stored":disk.read("dictionary.json",DICTIONARY_MAXIMUM)?,
            "portable":disk.read("dictionary-return-v1.json",DICTIONARY_MAXIMUM)?,"enabled":null}),
        ),
        "statistics.document" => {
            let mut values = serde_json::Map::new();
            for (role, file) in [("days", "stats.json"), ("syncState", "sync-state.json")] {
                let value = match disk.read(file, MAXIMUM) {
                    Ok(raw) => json!(raw),
                    Err(Error::Invalid) => json!({"invalid":true}),
                    Err(Error::Unavailable) => json!({"unavailable":true}),
                };
                values.insert(role.into(), value);
            }
            Ok(
                json!({"stored":disk.read("statistics-journal-v1.json",MAXIMUM)?,"continuity":{"source":"windows","values":values}}),
            )
        }
        _ => Err(Error::Invalid),
    }
}
fn write_target<'a>(
    key: &str,
    current: &Value,
    next: &'a Value,
) -> Result<(&'static str, &'a str), Error> {
    if settings_document::handles(key) {
        if !exact(next, &["source", "values"]) || next["source"] != "windows" {
            return Err(Error::Invalid);
        }
        let raw = next["values"].as_str().ok_or(Error::Invalid)?;
        settings_document::validate(key, current["values"].as_str(), raw)?;
        return Ok(("settings.json", raw));
    }
    let stored = next["stored"].as_str().ok_or(Error::Invalid)?;
    if let Some(file) = journal(key) {
        if !exact(next, &["source", "stored"]) || next["source"] != "windows" {
            return Err(Error::Invalid);
        }
        return Ok((file, stored));
    }
    match key {
        "dictionary.document"
            if exact(next, &["layout", "stored", "portable", "enabled"])
                && next["layout"] == "combined"
                && next["enabled"].is_null()
                && next["portable"] == current["portable"]
                && stored.len() <= DICTIONARY_MAXIMUM =>
        {
            Ok(("dictionary.json", stored))
        }
        "statistics.document"
            if exact(next, &["stored", "continuity"])
                && next["continuity"] == current["continuity"] =>
        {
            Ok(("statistics-journal-v1.json", stored))
        }
        _ => Err(Error::Invalid),
    }
}
fn execute(disk: &impl Disk, method: &str, params: &Value) -> Result<Value, Error> {
    let _guard = WRITER.lock().map_err(|_| Error::Unavailable)?;
    let key = params["key"].as_str().ok_or(Error::Invalid)?;
    if !matches!(method, "storage.read" | "storage.write") {
        return Err(Error::Invalid);
    }
    let current = read(disk, key)?;
    let current_revision = revision(&current)?;
    if method == "storage.read" {
        return available(current);
    }
    let expected = params["expectedRevision"].as_str().ok_or(Error::Invalid)?;
    if expected != current_revision {
        return Ok(json!({"status":"conflict"}));
    }
    let next = &params["value"];
    revision(next)?;
    let (file, raw) = write_target(key, &current, next)?;
    if raw.len() > MAXIMUM {
        return Err(Error::Invalid);
    }
    disk.write(file, raw)?;
    let observed = read(disk, key)?;
    if observed != *next {
        return Ok(json!({"status":"conflict"}));
    }
    available(observed)
}
pub fn invoke(disk: &impl Disk, method: &str, params: &Value) -> Value {
    execute(disk,method,params).unwrap_or_else(|error| json!({"status":match error {Error::Invalid=>"invalid",Error::Unavailable=>"unavailable"}}))
}
