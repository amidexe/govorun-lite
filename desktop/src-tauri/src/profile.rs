use std::path::{Path, PathBuf};

/// Identifiers are a closed list, never paths supplied by a renderer or CLI.
#[derive(Clone, Copy)]
pub enum Profile { Ordinary, Test }
impl Profile {
    pub fn from_identifier(identifier: &str) -> Result<Self, &'static str> {
        match identifier {
            "com.govorun.app" => Ok(Self::Ordinary),
            "com.govorun.app.testbackend" => Ok(Self::Test),
            _ => Err("storage_invalid_identity"),
        }
    }
    pub fn identifier(self) -> &'static str {
        match self { Self::Ordinary => "com.govorun.app", Self::Test => "com.govorun.app.testbackend" }
    }
    pub fn windows_data_name(self) -> &'static str {
        match self { Self::Ordinary => "Govorun", Self::Test => "Govorun.TestBackend" }
    }
    pub fn windows_entropy(self) -> &'static str {
        match self {
            Self::Ordinary => "Govorun.Windows.v2",
            Self::Test => "Govorun.Windows.v2.TestBackend",
        }
    }
    /// Preserve each pre-refactor namespace; isolated profiles ignore overrides.
    pub fn windows_data_directory(self, configured: Option<&std::ffi::OsStr>,
        local: Option<&Path>) -> Result<PathBuf, &'static str> {
        let configured = configured.filter(|value| !value.to_string_lossy().trim().is_empty());
        let root = match (self, configured) {
            (Self::Ordinary, Some(value)) => PathBuf::from(value),
            _ => local.ok_or("storage_unavailable")?.join(self.windows_data_name()),
        };
        std::path::absolute(root).map_err(|_| "storage_unavailable")
    }

}

