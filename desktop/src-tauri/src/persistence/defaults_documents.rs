//! Raw macOS Defaults document layouts. All product interpretation stays in Core.
use super::file_documents::Error;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};
const MAXIMUM: usize = 8 * 1_048_576;
static WRITER: Mutex<()> = Mutex::new(());
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Data(Vec<u8>),
    Scalar(Value),
    Other,
}
pub type Snapshot = BTreeMap<String, Cell>;
pub type Changes = BTreeMap<String, Option<Cell>>;
pub trait Domain: Send + Sync {
    fn read(&self) -> Result<Snapshot, Error>;
    fn write(&self, changes: Changes) -> Result<(), Error>;
    fn integer(&self, key: &str) -> i64;
    fn boolean(&self, key: &str) -> bool;
}
impl<T: Domain> Domain for std::sync::Arc<T> {
    fn read(&self) -> Result<Snapshot, Error> {
        (**self).read()
    }
    fn write(&self, changes: Changes) -> Result<(), Error> {
        (**self).write(changes)
    }
    fn integer(&self, key: &str) -> i64 {
        (**self).integer(key)
    }
    fn boolean(&self, key: &str) -> bool {
        (**self).boolean(key)
    }
}
fn specs() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../shared/core/contracts/storage-keys.json"))
            .expect("bundled storage locations")
    })
}
fn text(values: &Snapshot, key: &str, maximum: usize) -> Result<Value, Error> {
    match values.get(key) {
        None => Ok(Value::Null),
        Some(Cell::Data(data)) if data.len() <= maximum => {
            Ok(json!(std::str::from_utf8(data).map_err(|_| Error::Invalid)?))
        }
        _ => Err(Error::Invalid),
    }
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|v| v.len() == keys.len() && keys.iter().all(|k| v.contains_key(*k)))
}
fn fingerprint(value: &Value) -> Result<String, Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Invalid)?;
    if bytes.len() > MAXIMUM {
        return Err(Error::Invalid);
    }
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn available(value: Value) -> Result<Value, Error> {
    Ok(json!({"status":"available","revision":fingerprint(&value)?,"value":value}))
}
/// Used under the Foundation domain transaction gate, without recursive locking.
pub(super) fn observed(domain: &impl Domain, key: &str) -> Result<Value, Error> {
    let spec = &specs()["documents"][key]["macos"];
    available(read(domain, &domain.read()?, key, spec)?)
}
fn dictionary(values: &Snapshot) -> Result<Value, Error> {
    let enabled = match values.get("wordDictionaryEnabled") {
        None => false,
        Some(Cell::Scalar(Value::Bool(flag))) => *flag,
        Some(Cell::Scalar(Value::Number(number))) => number.as_f64().is_some_and(|x| x != 0.0),
        _ => return Err(Error::Invalid),
    };
    Ok(
        json!({"layout":"split","stored":text(values,"wordDictionary",2*1_048_576)?,
        "portable":text(values,"govorunDictionaryReturnV1",2*1_048_576)?,"enabled":enabled}),
    )
}
fn statistics(domain: &impl Domain, values: &Snapshot, spec: &Value) -> Result<Value, Error> {
    let mut total = Map::new();
    for (role, key) in spec["legacyNumbers"].as_object().ok_or(Error::Invalid)? {
        total.insert(
            role.clone(),
            json!(domain.integer(key.as_str().ok_or(Error::Invalid)?)),
        );
    }
    let legacy = |key: &str| text(values, key, MAXIMUM).unwrap_or_else(|_| json!({"invalid":true}));
    let mut facts = Map::from_iter([("total".into(), Value::Object(total))]);
    for (role, key) in spec["legacyRaw"].as_object().ok_or(Error::Invalid)? {
        facts.insert(role.clone(), legacy(key.as_str().ok_or(Error::Invalid)?));
    }
    let mut accounts: Map<String, Value> = Map::new();
    for (role, prefix) in spec["legacyAccountPrefixes"]
        .as_object()
        .ok_or(Error::Invalid)?
    {
        let prefix = prefix.as_str().ok_or(Error::Invalid)?;
        for key in values.keys() {
            if let Some(id) = key.strip_prefix(prefix) {
                let account = accounts.entry(id.to_string()).or_insert_with(|| json!({}));
                account[role] = if role == "resetPending" {
                    json!(domain.boolean(key))
                } else {
                    legacy(key)
                };
            }
        }
    }
    facts.insert("accounts".into(), Value::Object(accounts));
    Ok(
        json!({"stored":text(values,"govorunStatisticsJournalV1",MAXIMUM)?,"continuity":{"source":"macos","values":facts}}),
    )
}
fn scalar_keys(spec: &Value) -> Result<Vec<&str>, Error> {
    spec["scalarKeys"]
        .as_array()
        .ok_or(Error::Invalid)?
        .iter()
        .map(|x| x.as_str().ok_or(Error::Invalid))
        .collect()
}
fn raw_keys(spec: &Value) -> Result<Vec<&str>, Error> {
    spec["rawKeys"]
        .as_array()
        .ok_or(Error::Invalid)?
        .iter()
        .map(|x| x.as_str().ok_or(Error::Invalid))
        .collect()
}
fn settings(values: &Snapshot, spec: &Value) -> Result<Value, Error> {
    let mut selected = Map::new();
    for key in raw_keys(spec)? {
        if values.contains_key(key) {
            selected.insert(key.into(), text(values, key, MAXIMUM)?);
        }
    }
    for key in scalar_keys(spec)? {
        if let Some(cell) = values.get(key) {
            match cell {
                Cell::Scalar(value)
                    if value.is_string() || value.is_number() || value.is_boolean() =>
                {
                    selected.insert(key.into(), value.clone());
                }
                _ => return Err(Error::Invalid),
            }
        }
    }
    Ok(json!({"source":"macos","values":selected}))
}
fn valid_account(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && !id.bytes().any(|x| x < 32 || x == 127)
}
fn valid_slot(slot: &str) -> bool {
    slot.len() == 64
        && slot
            .bytes()
            .all(|x| x.is_ascii_digit() || (b'a'..=b'f').contains(&x))
}
fn enrollment(values: &Snapshot) -> Result<Value, Error> {
    let mut choices = Map::new();
    let mut requested = Map::new();
    for (key, cell) in values {
        if let Some(slot) = key.strip_prefix("govorunAccountDataSyncChoiceV1.") {
            if !valid_slot(slot) {
                return Err(Error::Invalid);
            }
            match cell {
                Cell::Scalar(Value::String(raw)) => {
                    choices.insert(slot.into(), json!(raw));
                }
                _ => return Err(Error::Invalid),
            }
        } else if let Some(slot) = key.strip_prefix("syncDecisionRequested.") {
            if !valid_slot(slot) {
                return Err(Error::Invalid);
            }
            match cell {
                Cell::Scalar(Value::Bool(flag)) => {
                    requested.insert(slot.into(), json!(flag));
                }
                _ => return Err(Error::Invalid),
            }
        }
    }
    Ok(json!({"source":"macos","stored":{"choices":choices,"requested":requested}}))
}
fn sync(values: &Snapshot, key: &str, spec: &Value) -> Result<Value, Error> {
    let prefix = spec["prefix"].as_str().ok_or(Error::Invalid)?;
    let mut stored = Map::new();
    for name in values.keys() {
        if let Some(id) = name.strip_prefix(prefix) {
            if !valid_account(id) {
                return Err(Error::Invalid);
            }
            stored.insert(id.into(), text(values, name, MAXIMUM)?);
        }
    }
    let mut result = json!({"source":"macos","stored":stored});
    if key == "sync.content" {
        result["mutationRevision"] = match values.get("govorunContentMutationGenerationV2") {
            None => Value::Null,
            Some(Cell::Scalar(value)) if value.as_i64().is_some_and(|x| x >= 0) => value.clone(),
            _ => return Err(Error::Invalid),
        };
    }
    Ok(result)
}
fn read(domain: &impl Domain, values: &Snapshot, key: &str, spec: &Value) -> Result<Value, Error> {
    match key {
        "dictionary.document" => dictionary(values),
        "statistics.document" => statistics(domain, values, spec),
        "sync.enrollment" => enrollment(values),
        "sync.content" | "sync.preferences" => sync(values, key, spec),
        "styles.document" | "selfHost.document" | "clipboard.document" | "hotkeys.document"
        | "recording.document" => settings(values, spec),
        _ => Err(Error::Invalid),
    }
}
fn data(raw: &Value, maximum: usize) -> Result<Cell, Error> {
    let text = raw.as_str().ok_or(Error::Invalid)?;
    if text.len() > maximum {
        return Err(Error::Invalid);
    }
    Ok(Cell::Data(text.as_bytes().to_vec()))
}
fn changes(key: &str, current: &Value, next: &Value, spec: &Value) -> Result<Changes, Error> {
    let mut changes = Changes::new();
    fingerprint(next)?;
    match key {
        "dictionary.document" => {
            if !exact(next, &["layout", "stored", "portable", "enabled"])
                || next["layout"] != "split"
                || !next["enabled"].is_boolean()
                || next["portable"] != current["portable"]
            {
                return Err(Error::Invalid);
            }
            changes.insert(
                "wordDictionary".into(),
                Some(data(&next["stored"], 2 * 1_048_576)?),
            );
            changes.insert(
                "wordDictionaryEnabled".into(),
                Some(Cell::Scalar(next["enabled"].clone())),
            );
        }
        "statistics.document" => {
            if !exact(next, &["stored", "continuity"])
                || next["continuity"] != current["continuity"]
            {
                return Err(Error::Invalid);
            }
            changes.insert(
                "govorunStatisticsJournalV1".into(),
                Some(data(&next["stored"], MAXIMUM)?),
            );
        }
        "sync.enrollment" => {
            if !exact(next, &["source", "stored"])
                || next["source"] != "macos"
                || !exact(&next["stored"], &["choices", "requested"])
            {
                return Err(Error::Invalid);
            }
            let choices = next["stored"]["choices"]
                .as_object()
                .ok_or(Error::Invalid)?;
            let requested = next["stored"]["requested"]
                .as_object()
                .ok_or(Error::Invalid)?;
            if current["stored"]["choices"]
                .as_object()
                .ok_or(Error::Invalid)?
                .keys()
                .any(|slot| !choices.contains_key(slot))
            {
                return Err(Error::Invalid);
            }
            for (slot, raw) in choices {
                if !valid_slot(slot) || !raw.is_string() {
                    return Err(Error::Invalid);
                }
                changes.insert(
                    format!("govorunAccountDataSyncChoiceV1.{slot}"),
                    Some(Cell::Scalar(raw.clone())),
                );
            }
            for (slot, flag) in requested {
                if !valid_slot(slot) || !flag.is_boolean() {
                    return Err(Error::Invalid);
                }
                changes.insert(
                    format!("syncDecisionRequested.{slot}"),
                    Some(Cell::Scalar(flag.clone())),
                );
            }
            for slot in current["stored"]["requested"]
                .as_object()
                .ok_or(Error::Invalid)?
                .keys()
            {
                if !requested.contains_key(slot) {
                    changes.insert(format!("syncDecisionRequested.{slot}"), None);
                }
            }
        }
        "sync.content" | "sync.preferences" => {
            let keys = if key == "sync.content" {
                vec!["source", "stored", "mutationRevision"]
            } else {
                vec!["source", "stored"]
            };
            if !exact(next, &keys) || next["source"] != "macos" {
                return Err(Error::Invalid);
            }
            let stored = next["stored"].as_object().ok_or(Error::Invalid)?;
            if current["stored"]
                .as_object()
                .ok_or(Error::Invalid)?
                .keys()
                .any(|id| !stored.contains_key(id))
            {
                return Err(Error::Invalid);
            }
            let prefix = spec["prefix"].as_str().ok_or(Error::Invalid)?;
            for (id, raw) in stored {
                if !valid_account(id) {
                    return Err(Error::Invalid);
                }
                changes.insert(format!("{prefix}{id}"), Some(data(raw, MAXIMUM)?));
            }
            if key == "sync.content" {
                let clock = &next["mutationRevision"];
                if clock.is_null() {
                    if !current["mutationRevision"].is_null() {
                        return Err(Error::Invalid);
                    }
                } else if clock.as_i64().is_some_and(|x| x >= 0) {
                    changes.insert(
                        "govorunContentMutationGenerationV2".into(),
                        Some(Cell::Scalar(clock.clone())),
                    );
                } else {
                    return Err(Error::Invalid);
                }
            }
        }
        _ => {
            if !exact(next, &["source", "values"]) || next["source"] != "macos" {
                return Err(Error::Invalid);
            }
            let raw = raw_keys(spec)?;
            let scalar = scalar_keys(spec)?;
            let selected = next["values"].as_object().ok_or(Error::Invalid)?;
            if selected
                .keys()
                .any(|k| !raw.contains(&k.as_str()) && !scalar.contains(&k.as_str()))
            {
                return Err(Error::Invalid);
            }
            for key in raw {
                changes.insert(
                    key.into(),
                    selected.get(key).map(|v| data(v, MAXIMUM)).transpose()?,
                );
            }
            for key in scalar {
                let value = selected.get(key);
                if value.is_some_and(|v| !v.is_string() && !v.is_number() && !v.is_boolean()) {
                    return Err(Error::Invalid);
                }
                changes.insert(key.into(), value.map(|v| Cell::Scalar(v.clone())));
            }
        }
    }
    Ok(changes)
}
fn execute(domain: &impl Domain, method: &str, params: &Value) -> Result<Value, Error> {
    let _guard = WRITER.lock().map_err(|_| Error::Unavailable)?;
    if !matches!(method, "storage.read" | "storage.write") {
        return Err(Error::Invalid);
    }
    let key = params["key"].as_str().ok_or(Error::Invalid)?;
    let spec = &specs()["documents"][key]["macos"];
    let values = domain.read()?;
    let current = read(domain, &values, key, spec)?;
    if method == "storage.read" {
        return available(current);
    }
    let expected = params["expectedRevision"].as_str().ok_or(Error::Invalid)?;
    if expected != fingerprint(&current)? {
        return Ok(json!({"status":"conflict"}));
    }
    let next = &params["value"];
    let changes = changes(key, &current, next, spec)?;
    domain.write(changes)?;
    let observed = read(domain, &domain.read()?, key, spec)?;
    if observed != *next {
        return Ok(json!({"status":"conflict"}));
    }
    available(observed)
}
pub fn invoke(domain: &impl Domain, method: &str, params: &Value) -> Value {
    execute(domain,method,params).unwrap_or_else(|error|json!({"status":match error {Error::Invalid=>"invalid",Error::Unavailable=>"unavailable"}}))
}
