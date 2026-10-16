use serde::Deserialize;
use std::{fs, io::Write, path::PathBuf, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

pub use govorun_core::boundary_snapshots::{SetupStep as Step, SetupProgress as Progress, SetupSnapshot as Snapshot, SetupChange as Change};
#[derive(Clone)]
pub struct Store { path: PathBuf, lock: Arc<Mutex<()>> }

impl Store {
    pub fn new(path: PathBuf) -> Self { Self { path, lock: Arc::new(Mutex::new(())) } }

    pub fn execute(&self, change: Option<Change>, reset: bool) -> Result<Snapshot, &'static str> {
        let _guard = self.lock.lock().map_err(|_| "setup_unavailable")?;
        let mut snapshot = Snapshot { version: 1, revision: 0, progress: None };
        if reset && change.is_some() { return Err("setup_invalid"); }
        match fs::read(&self.path) {
            Ok(bytes) => {
                let decoded = if bytes.len() <= 2048 { serde_json::from_slice::<Snapshot>(&bytes).ok() } else { None };
                if let Some(value) = decoded.filter(|value| value.version == 1 &&
                    value.revision < 9_007_199_254_740_990) { snapshot = value; }
                else if !reset { return Err("setup_invalid"); }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("setup_unavailable"),
        }
        if reset {
            // A cancelled save must not match the revision of a fresh setup.
            let epoch = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "setup_unavailable")?.as_millis();
            snapshot.revision = (snapshot.revision + 1).max(u64::try_from(epoch).map_err(|_| "setup_unavailable")?);
            snapshot.progress = Some(Progress { step: Step::Welcome });
        }
        if let Some(change) = &change {
            if reset || change.expected_revision != snapshot.revision { return Err("setup_stale"); }
            snapshot.revision += 1;
            snapshot.progress = Some(change.progress.clone());
        }
        if reset || change.is_some() {
            let parent = self.path.parent().ok_or("setup_unavailable")?;
            fs::create_dir_all(parent).map_err(|_| "setup_save_failed")?;
            let pending = parent.join("setup.pending");
            let mut file = fs::File::create(&pending).map_err(|_| "setup_save_failed")?;
            let bytes = serde_json::to_vec(&snapshot).map_err(|_| "setup_save_failed")?;
            file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|_| "setup_save_failed")?;
            drop(file);
            fs::rename(pending, &self.path).map_err(|_| "setup_save_failed")?;
        }
        Ok(snapshot)
    }
}
