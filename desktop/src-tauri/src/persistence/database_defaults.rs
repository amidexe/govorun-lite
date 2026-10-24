//! macOS documents in the base: every document key of the Defaults domain becomes a
//! row, and each document is its own source, so an outside write to one key moves only
//! that document again. Step 2 (2.1.112) added two sources that are not documents: the
//! diagnostic journal (before, every journal row rewrote the whole Defaults domain with
//! the sign-in markers in it) and the account's sign-in markers, which Windows keeps in
//! the base since 2.1.110. The old domain keeps the installation id, the own-server key
//! hints with the plain keys of the old app, and system values.
use super::{
    defaults_documents::{self as layout, Domain, Snapshot},
    document_db::{Cell, Changes, Db, Legacy, Notice, Rows, ANY_ACCOUNT, DOCUMENTS, FILE, OVERSIZED},
    file_documents::Error,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

const JOURNAL: &str = "journal";
/// The sign-in markers of the account; its credentials stay in their private file.
const ACCOUNT: &str = "account";
fn sources() -> Vec<&'static str> {
    DOCUMENTS.iter().copied().chain([JOURNAL, ACCOUNT]).collect()
}
struct Owned {
    document: &'static str,
    keys: Vec<String>,
    prefixes: Vec<String>,
}
fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../shared/core/contracts/storage-keys.json"))
            .expect("bundled storage locations")
    })
}
/// The marker that says a sign-in is stored.
fn session_marker() -> &'static str {
    table()["secrets"]["account.session"]["macos"]["defaults"]["sessionMarker"]
        .as_str()
        .expect("bundled session marker")
}
/// Exact keys and key prefixes of each moved source, from the storage key table.
fn owned() -> &'static [Owned] {
    static OWNED: OnceLock<Vec<Owned>> = OnceLock::new();
    OWNED.get_or_init(|| {
        let table = table();
        let text = |value: &Value| value.as_str().map(String::from);
        let markers = &table["secrets"]["account.session"]["macos"]["defaults"];
        DOCUMENTS
            .iter()
            .map(|document| {
                let spec = &table["documents"][document]["macos"];
                let (mut keys, mut prefixes) = (vec![], vec![]);
                for field in ["key", "portableKey", "enabledKey", "mutationClockKey"] {
                    keys.extend(text(&spec[field]));
                }
                for field in ["rawKeys", "scalarKeys"] {
                    keys.extend(spec[field].as_array().into_iter().flatten().filter_map(text));
                }
                for field in ["legacyNumbers", "legacyRaw"] {
                    keys.extend(spec[field].as_object().into_iter().flat_map(|v| v.values()).filter_map(text));
                }
                for field in ["prefix", "choicePrefix", "requestedPrefix"] {
                    prefixes.extend(text(&spec[field]));
                }
                prefixes.extend(
                    spec["legacyAccountPrefixes"].as_object().into_iter().flat_map(|v| v.values()).filter_map(text),
                );
                Owned { document, keys, prefixes }
            })
            .chain([
                Owned { document: JOURNAL, keys: vec!["diagnosticsEvents".into()], prefixes: vec![] },
                Owned {
                    document: ACCOUNT,
                    keys: markers.as_object().into_iter().flat_map(|v| v.values()).filter_map(text).collect(),
                    prefixes: vec![],
                },
            ])
            .collect()
    })
}
/// The source a Defaults key belongs to, if it moves at all.
pub fn document(key: &str) -> Option<&'static str> {
    owned()
        .iter()
        .find(|owned| {
            owned.keys.iter().any(|known| known == key)
                || owned.prefixes.iter().any(|prefix| key.starts_with(prefix.as_str()))
        })
        .map(|owned| owned.document)
}
pub fn moves(key: &str) -> bool {
    document(key).is_some()
}
fn stored(cell: layout::Cell) -> Result<Cell, Error> {
    match cell {
        layout::Cell::Data(bytes) => Ok(Cell::Bytes(bytes)),
        layout::Cell::Scalar(value) => Ok(Cell::Scalar(value)),
        layout::Cell::Other => Err(Error::Invalid),
    }
}
fn shown(cell: Cell) -> layout::Cell {
    match cell {
        Cell::Bytes(bytes) => layout::Cell::Data(bytes),
        // The Foundation adapter's marker for a value too large to read.
        Cell::Oversized => layout::Cell::Data(vec![0; OVERSIZED + 1]),
        Cell::Scalar(value) => layout::Cell::Scalar(value),
        Cell::Other => layout::Cell::Other,
    }
}

/// The old Defaults domain, read only.
pub struct OldDefaults<D>(pub D);
impl<D: Domain> Legacy for OldDefaults<D> {
    fn sources(&self) -> Vec<&'static str> {
        sources()
    }
    /// A sign-in or sign-out refreshes the good copy too, so a restored base never brings
    /// back an older one.
    fn holds_consent(&self, row: &str) -> bool {
        matches!(document(row), Some("styles.document" | "selfHost.document" | "sync.enrollment"))
            || row == session_marker()
    }
    fn rows(&self, source: &str) -> Result<Rows, Error> {
        if !sources().contains(&source) {
            return Err(Error::Invalid);
        }
        Ok(self
            .0
            .read()?
            .into_iter()
            .filter(|(key, _)| document(key) == Some(source))
            .map(|(key, cell)| {
                let cell = match cell {
                    layout::Cell::Data(bytes) if bytes.len() > OVERSIZED => Cell::Oversized,
                    layout::Cell::Data(bytes) => Cell::Bytes(bytes),
                    layout::Cell::Scalar(value) => Cell::Scalar(value),
                    layout::Cell::Other => Cell::Other,
                };
                (key, cell)
            })
            .collect())
    }
    fn fresh(&self) -> Vec<(&'static str, String, Cell)> {
        vec![("sync.enrollment", format!("syncDecisionRequested.{ANY_ACCOUNT}"), Cell::Scalar(Value::Bool(true)))]
    }
}

pub struct Documents<D> {
    db: Db<OldDefaults<D>>,
}
impl<D: Domain> Documents<D> {
    pub fn new(folder: PathBuf, defaults: D, notice: Option<Notice>) -> Self {
        Self { db: Db::new(folder.join(FILE), OldDefaults(defaults), notice) }
    }
    fn scalar(&self, key: &str) -> Result<Option<Value>, Error> {
        let source = document(key).ok_or(Error::Invalid)?;
        Ok(match self.db.read(source, key)? {
            Some(Cell::Scalar(value)) => Some(value),
            _ => None,
        })
    }
    fn snapshot(&self, sources: &[&str]) -> Result<Snapshot, Error> {
        Ok(self.db.rows(sources)?.into_iter().map(|(key, cell)| (key, shown(cell))).collect())
    }
    /// One atomic write; every key belongs to one of these sources.
    fn put(&self, sources: &[&str], changes: layout::Changes) -> Result<(), Error> {
        let changes = changes
            .into_iter()
            .map(|(key, cell)| {
                let source = document(&key).filter(|source| sources.contains(source)).ok_or(Error::Invalid)?;
                Ok((source, key, cell.map(stored).transpose()?))
            })
            .collect::<Result<Changes, _>>()?;
        self.db.write(&changes)
    }
}
/// NSString integerValue: optional spaces and sign, then the leading digits.
fn leading_integer(text: &str) -> i64 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |sum, digit| sum.saturating_mul(10).saturating_add((digit - b'0') as i64));
    if negative { -value } else { value }
}
impl<D: Domain> Domain for Documents<D> {
    /// The documents only: the journal and the sign-in markers have views of their own.
    fn read(&self) -> Result<Snapshot, Error> {
        self.snapshot(&DOCUMENTS)
    }
    fn write(&self, changes: layout::Changes) -> Result<(), Error> {
        self.put(&DOCUMENTS, changes)
    }
    /// NSUserDefaults integerForKey for the values the old app stored. These numbers
    /// only ever come from old versions: when the base cannot answer, the old domain does.
    fn integer(&self, key: &str) -> i64 {
        match self.scalar(key) {
            Ok(Some(Value::Bool(flag))) => flag as i64,
            Ok(Some(Value::Number(number))) => number.as_i64().unwrap_or_else(|| number.as_f64().unwrap_or(0.0) as i64),
            Ok(Some(Value::String(text))) => leading_integer(&text),
            Ok(_) => 0,
            Err(_) => self.db.legacy().0.integer(key),
        }
    }
    /// NSUserDefaults boolForKey: YES/true/a non-zero number.
    fn boolean(&self, key: &str) -> bool {
        match self.scalar(key) {
            Ok(Some(Value::Bool(flag))) => flag,
            Ok(Some(Value::Number(number))) => number.as_f64().is_some_and(|value| value != 0.0),
            Ok(Some(Value::String(text))) => {
                let text = text.trim_start().trim_start_matches(['+', '-']).trim_start_matches('0');
                text.chars().next().is_some_and(|first| matches!(first, 'Y' | 'y' | 'T' | 't' | '1'..='9'))
            }
            Ok(_) => false,
            Err(_) => self.db.legacy().0.boolean(key),
        }
    }
}

/// The diagnostic journal's view: one row, never the documents beside it.
pub struct Journal<D>(pub Arc<Documents<D>>);
impl<D: Domain> Domain for Journal<D> {
    fn read(&self) -> Result<Snapshot, Error> {
        self.0.snapshot(&[JOURNAL])
    }
    fn write(&self, changes: layout::Changes) -> Result<(), Error> {
        self.0.put(&[JOURNAL], changes)
    }
    fn integer(&self, key: &str) -> i64 {
        self.0.integer(key)
    }
    fn boolean(&self, key: &str) -> bool {
        self.0.boolean(key)
    }
}

static ACCOUNT_WRITER: Mutex<()> = Mutex::new(());
/// The account's domain after step 2: its sign-in markers are rows of the base; the
/// own-server key hints and the plain keys of the old app are read and removed in the old
/// domain they came from and are never copied into the base.
pub struct Account<D>(pub Arc<Documents<D>>);
impl<D: Domain> Domain for Account<D> {
    fn read(&self) -> Result<Snapshot, Error> {
        let mut values: Snapshot =
            self.0.db.legacy().0.read()?.into_iter().filter(|(key, _)| !moves(key)).collect();
        values.extend(self.0.snapshot(&[ACCOUNT])?);
        Ok(values)
    }
    fn write(&self, changes: layout::Changes) -> Result<(), Error> {
        let (markers, old): (layout::Changes, layout::Changes) =
            changes.into_iter().partition(|(key, _)| document(key) == Some(ACCOUNT));
        if old.keys().any(|key| moves(key)) {
            return Err(Error::Invalid);
        }
        if !markers.is_empty() {
            let _guard = ACCOUNT_WRITER.lock().map_err(|_| Error::Unavailable)?;
            // Only what changed: a profile refresh writes its own row, and only a sign-in
            // or sign-out refreshes the good copy.
            let current = self.0.db.rows(&[ACCOUNT])?;
            let mut rows = Changes::new();
            for (key, cell) in markers {
                let cell = cell.map(stored).transpose()?;
                if current.get(&key) != cell.as_ref() {
                    rows.push((ACCOUNT, key, cell));
                }
            }
            if !rows.is_empty() {
                self.0.db.write(&rows)?;
            }
        }
        if !old.is_empty() {
            self.0.db.legacy().0.write(old)?;
        }
        Ok(())
    }
    fn integer(&self, key: &str) -> i64 {
        if document(key) == Some(ACCOUNT) { self.0.integer(key) } else { self.0.db.legacy().0.integer(key) }
    }
    fn boolean(&self, key: &str) -> bool {
        if document(key) == Some(ACCOUNT) { self.0.boolean(key) } else { self.0.db.legacy().0.boolean(key) }
    }
}
