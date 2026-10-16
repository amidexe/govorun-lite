use sha2::{Digest, Sha256};
use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Component, Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

#[derive(Clone, Copy, serde::Serialize)]
pub struct ModelFile { pub name: &'static str, pub bytes: u64, pub sha256: &'static str }
pub struct Specification { pub revision: &'static str, pub files: &'static [ModelFile] }

// Model identity is independent of the application version. Updating the UI does
// not invalidate these files. Each revision is verified before it can be used.
pub const PRODUCTION: Specification = Specification {
    revision: "gigaam-v3-q8-78d63b47-silero4-9e2449e1",
    files: &[
        ModelFile { name: "gigaam-v3-e2e-rnnt-Q8_0.gguf", bytes: 273_724_832,
            sha256: "78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a" },
        ModelFile { name: "silero_vad.onnx", bytes: 643_854,
            sha256: "9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6" },
    ],
};

// Never follow directory links into another profile during migration/cleanup.
pub fn plain_path(path: &Path) -> Result<(), &'static str> {
    if !path.is_absolute() { return Err("model_invalid_path"); }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) { return Err("model_invalid_path"); }
        current.push(part);
        // A Windows volume prefix such as \\?\C: is not itself a filesystem
        // entry. Inspect it only once its root separator has been appended.
        if matches!(part, Component::Prefix(_)) { continue; }
        match fs::symlink_metadata(&current) {
            Ok(info) => {
                #[cfg(windows)]
                { use std::os::windows::fs::MetadataExt;
                  if info.file_attributes() & 0x400 != 0 { return Err("model_linked_path"); } }
                if info.file_type().is_symlink() { return Err("model_linked_path"); }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("model_storage_unavailable"),
        }
    }
    Ok(())
}

impl Specification {
    pub fn bytes(&self) -> u64 { self.files.iter().map(|file| file.bytes).sum() }

    pub fn identity(&self) -> String {
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_key(|file| file.name);
        let bytes = serde_json::to_vec(&files).expect("static model metadata");
        format!("{:x}", Sha256::digest(bytes))
    }

    pub fn verified(&self, directory: &Path) -> bool {
        if plain_path(directory).is_err() { return false; }
        self.files.iter().all(|spec| {
            let path = directory.join(spec.name);
            let Ok(info) = fs::symlink_metadata(&path) else { return false; };
            if !info.is_file() || info.len() != spec.bytes { return false; }
            #[cfg(windows)]
            { use std::os::windows::fs::MetadataExt;
              if info.file_attributes() & 0x400 != 0 { return false; } }
            let Ok(mut file) = File::open(&path) else { return false; };
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65_536];
            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => hash.update(&buffer[..count]),
                    Err(_) => return false,
                }
            }
            format!("{:x}", hash.finalize()) == spec.sha256
        })
    }

    /// Retain the reviewed model outside the app before updating or removing it.
    /// No existing file is changed until both incoming files have been verified.
    pub fn retain(&self, bundled: &Path, root: &Path) -> Result<PathBuf, &'static str> {
        plain_path(root)?;
        let destination = root.join(self.revision);
        plain_path(&destination)?;
        if self.verified(&destination) { return Ok(destination); }
        plain_path(bundled)?;
        if !self.verified(bundled) { return Err("model_files_missing_or_damaged"); }
        fs::create_dir_all(root).map_err(|_| "model_storage_unavailable")?;
        let lock_path = root.join(".migration.lock");
        plain_path(&lock_path)?;
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false)
            .open(lock_path).map_err(|_| "model_storage_unavailable")?;
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if lock.try_lock().is_ok() { break; }
            if Instant::now() >= deadline { return Err("model_storage_busy"); }
            std::thread::sleep(Duration::from_millis(10));
        }
        // The OS releases this lock even after a crash. A waiting migration must
        // recheck the completed pair before touching the destination directory.
        if self.verified(&destination) { return Ok(destination); }
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "model_storage_unavailable")?.as_nanos();
        let incoming = root.join(format!(".incoming-{}-{nonce}", std::process::id()));
        fs::create_dir(&incoming).map_err(|_| "model_storage_unavailable")?;
        let result = (|| {
            for spec in self.files {
                let source = bundled.join(spec.name);
                let target = incoming.join(spec.name);
                // Immutable hard links avoid another model-sized disk allocation
                // on the same volume. Updates always replace files, never write
                // into a retained inode. Cross-volume installs use a checked copy.
                if fs::hard_link(&source, &target).is_err() {
                    let mut input = File::open(&source).map_err(|_| "model_storage_unavailable")?;
                    let mut output = File::create(&target).map_err(|_| "model_storage_unavailable")?;
                    std::io::copy(&mut input, &mut output).map_err(|_| "model_storage_unavailable")?;
                    output.flush().and_then(|_| output.sync_all()).map_err(|_| "model_storage_unavailable")?;
                }
            }
            if !self.verified(&incoming) { return Err("model_files_missing_or_damaged"); }
            // A concurrent, already completed migration wins without replacement.
            if self.verified(&destination) { return Ok(destination.clone()); }
            if destination.exists() {
                // Retain a damaged copy for recovery; never recursively remove a
                // caller-supplied path or partially overwrite the selected model.
                let damaged = root.join(format!(".damaged-{nonce}"));
                fs::rename(&destination, &damaged).map_err(|_| "model_storage_unavailable")?;
                if fs::rename(&incoming, &destination).is_err() {
                    let _ = fs::rename(damaged, &destination);
                    return Err("model_storage_unavailable");
                }
            } else {
                fs::rename(&incoming, &destination).map_err(|_| "model_storage_unavailable")?;
            }
            Ok(destination.clone())
        })();
        if incoming.exists() { let _ = fs::remove_dir_all(incoming); }
        result
    }

    pub fn prepare(&self, bundled: &Path, root: &Path) -> Result<PathBuf, &'static str> {
        match self.retain(bundled, root) {
            Ok(path) => Ok(path),
            // Failure to create a cache must not disable a complete offline app.
            // Maintenance will require retain() to succeed before removing it.
            Err(_) if plain_path(bundled).is_ok() && self.verified(bundled) => Ok(bundled.to_owned()),
            Err(error) => Err(error),
        }
    }
}

pub fn model_root(local_data: &Path, identifier: &str) -> Result<PathBuf, &'static str> {
    if identifier.is_empty() || identifier.len() > 120 || identifier.starts_with('.')
        || identifier.ends_with('.') || identifier.contains("..")
        || !identifier.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-') {
        return Err("model_invalid_identity");
    }
    plain_path(local_data)?;
    Ok(local_data.join(format!("{identifier}.models")))
}

pub fn application_paths(resources: &Path, local_data: &Path, identifier: &str) -> Result<(PathBuf, PathBuf), &'static str> {
    let bundled = resources.join(if cfg!(target_os = "macos") { "Model" } else { "native/Models" });
    Ok((bundled, model_root(local_data, identifier)?))
}
