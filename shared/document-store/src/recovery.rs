//! Files around the documents base: the daily good copy, damage set aside beside it,
//! and the replacement base. Every file is synced before it is renamed into place,
//! and nothing is ever deleted except copies made here.
use super::{put, schema, Cell, Error, Fault, BACKUP_AGE};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
/// Exists, absent, or unknown (a stat error is never read as "deleted").
pub fn present(path: &Path) -> Result<bool, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Error::Unavailable),
    }
}
/// Windows flushes only a handle opened for writing.
fn synced(path: &Path) -> Result<(), Error> {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| Error::Unavailable)
}
/// Renames into a place that must still be free: nothing is ever replaced.
fn place(from: &Path, to: &Path) -> Result<(), Error> {
    if present(to)? {
        return Err(Error::Unavailable);
    }
    std::fs::rename(from, to).map_err(|_| Error::Unavailable)
}
fn remove_ours(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(sibling(path, suffix));
    }
}
/// A broken file is kept beside the base with the date, never deleted.
pub fn set_aside(path: &Path) -> Result<(), Error> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    for suffix in ["", "-wal", "-shm"] {
        let from = sibling(path, suffix);
        if !present(&from)? {
            continue;
        }
        let mut to = sibling(&from, &format!(".broken-{stamp}"));
        for attempt in 2.. {
            if !present(&to)? {
                break;
            }
            to = sibling(&from, &format!(".broken-{stamp}-{attempt}"));
        }
        std::fs::rename(&from, to).map_err(|_| Error::Unavailable)?;
    }
    Ok(())
}
/// Damage was set aside earlier but no base replaced it (a crash in between).
pub fn interrupted(path: &Path) -> Result<bool, Error> {
    if present(path)? {
        return Ok(false);
    }
    if present(&sibling(path, ".bak"))? {
        return Ok(true);
    }
    let (Some(folder), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) else {
        return Ok(false);
    };
    let prefix = format!("{name}.broken-");
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        // The first start: the data folder does not exist yet.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(Error::Unavailable),
    };
    Ok(entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_str().is_some_and(|file| file.starts_with(&prefix))))
}
/// The last good copy; `always` after a consent or sync choice changed.
pub fn back_up(connection: &Connection, path: &Path, always: bool) {
    let backup = sibling(path, ".bak");
    let recent = std::fs::metadata(&backup)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.elapsed().ok())
        .is_some_and(|age| age < BACKUP_AGE);
    let temporary = sibling(path, ".bak-tmp");
    let Some(target) = temporary.to_str().filter(|_| always || !recent) else {
        return;
    };
    let _ = std::fs::remove_file(&temporary);
    let done = connection.execute("VACUUM INTO ?1", [target]).is_ok()
        && synced(&temporary).is_ok()
        && std::fs::rename(&temporary, &backup).is_ok();
    if !done {
        let _ = std::fs::remove_file(&temporary);
    }
}
pub enum Replaced {
    Restored,
    Recreated,
}
/// Damage: the broken base goes aside and the last good copy comes back. Without a
/// good copy a new base starts empty with `fresh` rows, built under a temporary name
/// so a crash can never leave an empty base that looks like a first start.
pub fn replace(
    path: &Path,
    fresh: Vec<(&'static str, String, Cell)>,
    open: impl Fn(&Path) -> Result<Connection, Fault>,
) -> Result<(Connection, Replaced), Fault> {
    set_aside(path)?;
    let backup = sibling(path, ".bak");
    if present(&backup)? {
        let temporary = sibling(path, ".restore-tmp");
        let _ = std::fs::remove_file(&temporary);
        std::fs::copy(&backup, &temporary).map_err(|_| Fault::Failed(Error::Unavailable))?;
        synced(&temporary)?;
        place(&temporary, path)?;
        match open(path) {
            Ok(connection) => return Ok((connection, Replaced::Restored)),
            Err(Fault::Broken) => {
                // Only our copy goes; the broken copy itself stays aside.
                remove_ours(path);
                set_aside(&backup)?;
            }
            Err(fault) => return Err(fault),
        }
    }
    let temporary = sibling(path, ".new-tmp");
    remove_ours(&temporary);
    {
        let connection = Connection::open(&temporary)?;
        schema(&connection)?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("INSERT INTO meta(key,value) VALUES('recreated',1) ON CONFLICT(key) DO UPDATE SET value=1", [])?;
        for (source, name, cell) in fresh {
            put(&transaction, source, &name, Some(&cell))?;
        }
        transaction.commit()?;
    }
    synced(&temporary)?;
    place(&temporary, path)?;
    Ok((open(path)?, Replaced::Recreated))
}
