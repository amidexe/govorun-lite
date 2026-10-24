//! Windows documents in the base. The shared settings.json becomes one row per owner:
//! each document reads and writes only its own copy, the sign-in only its marker. The
//! diagnostic journal joined them in 2.1.112, as on macOS.
use super::{
    document_db::{Cell, Db, Legacy, Notice, Rows, ANY_ACCOUNT, FILE, OVERSIZED},
    file_documents::{Disk, Error},
};
use std::{io::Read, path::PathBuf};

const SETTINGS: &str = "settings.json";
/// Owners of a settings.json copy; `account` holds the sign-out marker.
pub const SETTINGS_OWNERS: [&str; 6] = [
    "styles.document",
    "selfHost.document",
    "hotkeys.document",
    "recording.document",
    "clipboard.document",
    "account",
];
const FILES: [&str; 10] = [
    SETTINGS,
    "diagnostics.log",
    "dictionary.json",
    "dictionary-return-v1.json",
    "statistics-journal-v1.json",
    "stats.json",
    "sync-state.json",
    "content-journal.json",
    "preference-journal.json",
    "account-data-sync.json",
];

/// The old data folder, read only.
pub struct OldFiles(pub PathBuf);
fn io(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::InvalidData {
        Error::Invalid
    } else {
        Error::Unavailable
    }
}
impl Legacy for OldFiles {
    fn sources(&self) -> Vec<&'static str> {
        FILES.to_vec()
    }
    fn holds_consent(&self, row: &str) -> bool {
        matches!(row, "styles.document/settings.json" | "selfHost.document/settings.json" | "account-data-sync.json")
    }
    fn rows(&self, source: &str) -> Result<Rows, Error> {
        if !FILES.contains(&source) {
            return Err(Error::Invalid);
        }
        let file = match std::fs::File::open(self.0.join(source)) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Rows::new()),
            Err(error) => return Err(io(error)),
        };
        let cell = if file.metadata().map_err(io)?.len() > OVERSIZED as u64 {
            Cell::Oversized
        } else {
            let mut bytes = Vec::new();
            file.take(OVERSIZED as u64 + 1).read_to_end(&mut bytes).map_err(io)?;
            if bytes.len() > OVERSIZED { Cell::Oversized } else { Cell::Bytes(bytes) }
        };
        Ok(if source == SETTINGS {
            SETTINGS_OWNERS.iter().map(|owner| (format!("{owner}/{SETTINGS}"), cell.clone())).collect()
        } else {
            Rows::from([(source.to_string(), cell)])
        })
    }
    fn fresh(&self) -> Vec<(&'static str, String, Cell)> {
        let asked = format!("{{\"requested:{ANY_ACCOUNT}\":1}}");
        vec![("account-data-sync.json", "account-data-sync.json".into(), Cell::Bytes(asked.into_bytes()))]
    }
}

pub struct Documents {
    db: Db<OldFiles>,
}
impl Documents {
    pub fn new(root: PathBuf, notice: Option<Notice>) -> Self {
        Self { db: Db::new(root.join(FILE), OldFiles(root), notice) }
    }
    /// The view one owner has: `settings.json` means its own copy.
    pub fn scope<'a>(&'a self, owner: &'a str) -> Scoped<'a> {
        Scoped { db: &self.db, owner: SETTINGS_OWNERS.iter().find(|known| **known == owner).copied() }
    }
}
pub struct Scoped<'a> {
    db: &'a Db<OldFiles>,
    owner: Option<&'static str>,
}
impl Scoped<'_> {
    fn row(&self, name: &str) -> Result<(&'static str, String), Error> {
        let source = FILES.iter().find(|file| **file == name).copied().ok_or(Error::Invalid)?;
        if source != SETTINGS {
            return Ok((source, name.into()));
        }
        let owner = self.owner.ok_or(Error::Invalid)?;
        Ok((SETTINGS, format!("{owner}/{SETTINGS}")))
    }
}
impl Disk for Scoped<'_> {
    fn read(&self, name: &str, maximum: usize) -> Result<Option<String>, Error> {
        let (source, row) = self.row(name)?;
        match self.db.read(source, &row)? {
            None => Ok(None),
            Some(Cell::Bytes(bytes)) if bytes.len() <= maximum => {
                String::from_utf8(bytes).map(Some).map_err(|_| Error::Invalid)
            }
            Some(_) => Err(Error::Invalid),
        }
    }
    fn write(&self, name: &str, raw: &str) -> Result<(), Error> {
        let (source, row) = self.row(name)?;
        self.db.write(&vec![(source, row, Some(Cell::Bytes(raw.as_bytes().to_vec())))])
    }
}
