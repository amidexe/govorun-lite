//! Physical Windows namespaces; private acceptance cannot open either product profile.
use crate::profile::Profile;
use std::{ffi::OsStr, path::{Path, PathBuf}};

pub struct Namespace {
    pub root: PathBuf,
    pub entropy: &'static str,
}

pub fn select(identity: &str, configured: Option<&OsStr>, local: Option<&Path>,
    private_fixture: bool) -> Result<Namespace, &'static str> {
    if private_fixture && identity == "local.govorun.tauri-acceptance" {
        return Ok(Namespace {
            root: local.ok_or("storage_unavailable")?.join("Govorun.TauriAcceptance"),
            entropy: "Govorun.Windows.TauriAcceptance",
        });
    }
    let profile = Profile::from_identifier(identity).map_err(|_| "storage_unavailable")?;
    Ok(Namespace {
        root: profile.windows_data_directory(configured, local)?,
        entropy: profile.windows_entropy(),
    })
}
