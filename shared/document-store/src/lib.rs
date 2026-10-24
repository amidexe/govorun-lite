//! One SQLite file under an application's documents, shared by desktop (2.1.110) and
//! Android. Each document owns its rows, so a value one feature cannot read no longer
//! disables another one. Old files are never written: they are copied byte for byte
//! once, and copied again when an older version changed them since (owner, 23 Oct 2026).
//! A damaged base comes back from the last good copy; without one it starts empty:
//! old files hold old consent and never fill a recovered base. Dictation never waits
//! on any of it: every failure reads as an unavailable document.
pub mod recovery;
use recovery::{back_up, interrupted, replace, Replaced};
use rusqlite::{Connection, ErrorCode, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

pub const FILE: &str = "govorun.db";
/// Raise it to copy every old source again, for example after a faulty move.
const MOVE_VERSION: i64 = 1;
pub const BACKUP_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY: Duration = Duration::from_secs(10);
/// The base could not be opened at its last try: settings show one calm line.
static UNAVAILABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn base_unavailable() -> bool {
    UNAVAILABLE.load(std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Bytes(Vec<u8>),
    Oversized,
    Scalar(serde_json::Value),
    Other,
}
pub type Rows = BTreeMap<String, Cell>;
/// (source, row, new value or removal) of one atomic write.
pub type Changes = Vec<(&'static str, String, Option<Cell>)>;
pub type Notice = Box<dyn Fn(&'static str) + Send + Sync>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
}

pub trait Legacy: Send + Sync {
    /// Every old source, moved when the base opens.
    fn sources(&self) -> Vec<&'static str>;
    /// Every row an old source holds now; empty when the source does not exist.
    fn rows(&self, source: &str) -> Result<Rows, Error>;
    /// A consent or sync choice: its change refreshes the good copy at once.
    fn holds_consent(&self, row: &str) -> bool;
    /// (source, row) a base recreated without a good copy starts with: the lost sync
    /// choice is asked once, as on a new computer.
    fn fresh(&self) -> Vec<(&'static str, String, Cell)>;
    /// An old source that reads empty after its move was damaged, not cleared: Android
    /// reads a broken preference file as empty. Desktop sources report damage as errors.
    fn empty_after_move_is_damage(&self) -> bool {
        false
    }
}

pub enum Fault {
    Broken,
    /// Another process holds the base; a later call tries again.
    Busy,
    Failed(Error),
}
impl From<rusqlite::Error> for Fault {
    fn from(error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => Fault::Broken,
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => Fault::Busy,
            _ => Fault::Failed(Error::Unavailable),
        }
    }
}
impl From<Error> for Fault {
    fn from(error: Error) -> Self {
        Fault::Failed(error)
    }
}

#[derive(Default)]
struct State {
    connection: Option<Connection>,
    checked: BTreeSet<String>,
    notices: Vec<&'static str>,
    reported: BTreeSet<&'static str>,
    damaged: bool,
    /// At most one replacement per launch: a disk that keeps breaking files must not
    /// fill itself with copies set aside.
    replaced: bool,
    /// A base another process holds is not waited for on every call.
    retry_after: Option<Instant>,
}
impl State {
    fn notice(&mut self, code: &'static str) {
        if self.reported.insert(code) {
            self.notices.push(code);
        }
    }
}

pub struct Db<L> {
    path: PathBuf,
    legacy: L,
    state: Mutex<State>,
    notice: Option<Notice>,
}

pub fn schema(connection: &Connection) -> Result<(), Fault> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS entries(name TEXT PRIMARY KEY NOT NULL, source TEXT NOT NULL,
            kind INTEGER NOT NULL, value BLOB) WITHOUT ROWID;
         CREATE TABLE IF NOT EXISTS moved(source TEXT PRIMARY KEY NOT NULL,
            fingerprint TEXT NOT NULL) WITHOUT ROWID;
         CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY NOT NULL,
            value INTEGER NOT NULL) WITHOUT ROWID;",
    )?;
    let version: Option<i64> = connection
        .query_row("SELECT value FROM meta WHERE key='moveVersion'", [], |row| row.get(0))
        .optional()?;
    // A newer version's number stays: it already moved under its own rules.
    if version.is_none_or(|version| version < MOVE_VERSION) {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM moved", [])?;
        transaction.execute(
            "INSERT INTO meta(key,value) VALUES('moveVersion',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [MOVE_VERSION],
        )?;
        transaction.commit()?;
    }
    Ok(())
}
fn open(path: &Path) -> Result<Connection, Fault> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| Fault::Failed(Error::Unavailable))?;
    }
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(1))?;
    let check: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(Fault::Broken);
    }
    let _: String = connection.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    schema(&connection)?;
    Ok(connection)
}

fn encode(cell: &Cell) -> Result<(i64, Option<Vec<u8>>), Error> {
    Ok(match cell {
        Cell::Bytes(bytes) => (0, Some(bytes.clone())),
        Cell::Oversized => (1, None),
        Cell::Scalar(value) => (2, Some(serde_json::to_vec(value).map_err(|_| Error::Invalid)?)),
        Cell::Other => (3, None),
    })
}
fn decode(kind: i64, value: Option<Vec<u8>>) -> Result<Cell, Error> {
    Ok(match (kind, value) {
        (0, Some(bytes)) => Cell::Bytes(bytes),
        (0, None) => Cell::Bytes(vec![]),
        (1, _) => Cell::Oversized,
        (2, Some(bytes)) => Cell::Scalar(serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?),
        (3, _) => Cell::Other,
        _ => return Err(Error::Invalid),
    })
}
fn fingerprint(rows: &Rows) -> Result<String, Error> {
    let mut hash = Sha256::new();
    for (name, cell) in rows {
        let (kind, value) = encode(cell)?;
        let value = value.unwrap_or_default();
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update(kind.to_le_bytes());
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(&value);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn put(connection: &Connection, source: &str, name: &str, cell: Option<&Cell>) -> Result<(), Fault> {
    match cell {
        None => {
            connection.execute("DELETE FROM entries WHERE name=?1", [name])?;
        }
        Some(cell) => {
            let (kind, value) = encode(cell)?;
            connection.execute(
                "INSERT INTO entries(name,source,kind,value) VALUES(?1,?2,?3,?4)
                 ON CONFLICT(name) DO UPDATE SET source=excluded.source,kind=excluded.kind,value=excluded.value",
                rusqlite::params![name, source, kind, value],
            )?;
        }
    }
    Ok(())
}
fn recorded(connection: &Connection, source: &str) -> Result<Option<String>, Fault> {
    Ok(connection
        .query_row("SELECT fingerprint FROM moved WHERE source=?1", [source], |row| row.get(0))
        .optional()?)
}

impl<L: Legacy> Db<L> {
    pub fn new(path: PathBuf, legacy: L, notice: Option<Notice>) -> Self {
        Self { path, legacy, state: Mutex::new(State::default()), notice }
    }
    pub fn legacy(&self) -> &L {
        &self.legacy
    }
    /// Copies an old source on its first use in this launch when it is new or changed.
    fn ensure(&self, state: &mut State, source: &str) -> Result<(), Fault> {
        if state.checked.contains(source) {
            return Ok(());
        }
        let connection = state.connection.as_mut().ok_or(Fault::Failed(Error::Unavailable))?;
        let before = recorded(connection, source)?;
        let recreated: Option<i64> = connection
            .query_row("SELECT value FROM meta WHERE key='recreated'", [], |row| row.get(0))
            .optional()?;
        let rows = match self.legacy.rows(source) {
            Ok(rows) if rows.is_empty() && before.is_some() && self.legacy.empty_after_move_is_damage() => {
                state.checked.insert(source.into());
                return Ok(());
            }
            Ok(rows) => rows,
            // Before its move an unreadable old file fails exactly as before and is
            // tried again; once moved, or in a recreated base, the base works without it.
            Err(_) if before.is_some() || recreated == Some(1) => {
                state.checked.insert(source.into());
                return Ok(());
            }
            Err(error) => return Err(Fault::Failed(error)),
        };
        let now = fingerprint(&rows)?;
        if before.as_deref() != Some(now.as_str()) {
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            // Another instance may have moved it meanwhile; its later writes stay.
            let current = recorded(&transaction, source)?;
            if current.as_deref() != Some(now.as_str()) {
                // A recreated base never takes old files: their consent predates the
                // person's later choices. Only a later change by an older version moves.
                let first = current.is_none() && recreated == Some(1);
                if !first {
                    transaction.execute("DELETE FROM entries WHERE source=?1", [source])?;
                    for (name, cell) in &rows {
                        put(&transaction, source, name, Some(cell))?;
                    }
                }
                transaction.execute(
                    "INSERT INTO moved(source,fingerprint) VALUES(?1,?2)
                     ON CONFLICT(source) DO UPDATE SET fingerprint=excluded.fingerprint",
                    [source, now.as_str()],
                )?;
                transaction.commit()?;
                if current.is_some() {
                    state.notice("storage_legacy_reimported");
                } else if !first && !rows.is_empty() {
                    state.notice("storage_moved");
                }
            }
        }
        state.checked.insert(source.into());
        Ok(())
    }
    fn connect(&self, state: &mut State) -> Result<(), Fault> {
        if state.connection.is_some() {
            return Ok(());
        }
        if state.retry_after.is_some_and(|after| Instant::now() < after) {
            return Err(Fault::Failed(Error::Unavailable));
        }
        let opened = if state.damaged || interrupted(&self.path)? { Err(Fault::Broken) } else { open(&self.path) };
        let connection = match opened {
            Err(Fault::Broken) if !state.replaced => {
                state.replaced = true;
                let (connection, how) = replace(&self.path, self.legacy.fresh(), open)?;
                state.notice(match how {
                    Replaced::Restored => "storage_restored_from_backup",
                    Replaced::Recreated => "storage_database_replaced",
                });
                connection
            }
            Err(Fault::Busy) => {
                state.retry_after = Some(Instant::now() + RETRY);
                return Err(Fault::Busy);
            }
            Err(Fault::Broken) => return Err(Fault::Failed(Error::Unavailable)),
            other => other?,
        };
        state.damaged = false;
        state.retry_after = None;
        state.connection = Some(connection);
        state.checked.clear();
        // Every old source moves before the good copy is taken, so a copy never
        // predates the move. A source that cannot be read now is tried on its use.
        for source in self.legacy.sources() {
            if let Err(Fault::Broken) = self.ensure(state, source) {
                return Err(Fault::Broken);
            }
        }
        if let Some(connection) = &state.connection {
            back_up(connection, &self.path, false);
        }
        Ok(())
    }
    fn run<T>(
        &self,
        sources: &[&str],
        operation: &dyn Fn(&mut Connection) -> Result<T, Fault>,
    ) -> Result<T, Error> {
        let (result, notices) = {
            let mut state = self.state.lock().map_err(|_| Error::Unavailable)?;
            let connected = self.connect(&mut state);
            UNAVAILABLE.store(connected.is_err(), std::sync::atomic::Ordering::Relaxed);
            let mut result = connected.and_then(|()| {
                for source in sources {
                    self.ensure(&mut state, source)?;
                }
                operation(state.connection.as_mut().ok_or(Fault::Failed(Error::Unavailable))?)
            });
            if matches!(result, Err(Fault::Broken)) {
                // Damage found while working: the next call sets the base aside and
                // brings back the good copy. Offline dictation never waits on it.
                state.connection = None;
                state.checked.clear();
                state.damaged = true;
                result = Err(Fault::Failed(Error::Unavailable));
            }
            (result, std::mem::take(&mut state.notices))
        };
        // Outside the lock: a journal listener may read documents again.
        if let Some(notice) = &self.notice {
            for code in notices {
                notice(code);
            }
        }
        result.map_err(|fault| match fault {
            Fault::Failed(error) => error,
            Fault::Broken | Fault::Busy => Error::Unavailable,
        })
    }
    pub fn read(&self, source: &str, name: &str) -> Result<Option<Cell>, Error> {
        self.run(&[source], &|connection| {
            let row: Option<(i64, Option<Vec<u8>>)> = connection
                .query_row("SELECT kind,value FROM entries WHERE name=?1", [name], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?;
            Ok(row.map(|(kind, value)| decode(kind, value)).transpose()?)
        })
    }
    /// The rows of these sources, each moved first.
    pub fn rows(&self, sources: &[&str]) -> Result<Rows, Error> {
        self.run(sources, &|connection| {
            let mut statement = connection.prepare("SELECT name,kind,value,source FROM entries")?;
            let mut rows = Rows::new();
            let mut cursor = statement.query([])?;
            while let Some(row) = cursor.next()? {
                let source: String = row.get(3)?;
                if sources.contains(&source.as_str()) {
                    rows.insert(row.get(0)?, decode(row.get(1)?, row.get(2)?)?);
                }
            }
            Ok(rows)
        })
    }
    /// Every change of one call lands together or not at all.
    pub fn write(&self, changes: &Changes) -> Result<(), Error> {
        let mut sources: Vec<&str> = changes.iter().map(|(source, _, _)| *source).collect();
        sources.dedup();
        let consent = changes.iter().any(|(_, name, _)| self.legacy.holds_consent(name));
        self.run(&sources, &|connection| {
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            for (source, name, cell) in changes {
                put(&transaction, source, name, cell.as_ref())?;
            }
            transaction.commit()?;
            // A changed consent or sync choice is in the good copy at once, so a
            // restore never brings back an older one.
            back_up(connection, &self.path, consent);
            Ok(())
        })
    }
}
