//! Explicit removal of Govorun from this Mac: its data, the model cache and the
//! application itself, once the process has exited.
use crate::{profile::Profile, windows};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager, WebviewWindow};

/// Waits for the application, never deletes beside a running process. Arguments are
/// positional, so no path or identifier is ever interpreted as shell syntax.
pub const SCRIPT: &str = r#"
n=0
while /bin/kill -0 "$1" 2>/dev/null; do
  n=$((n+1)); [ "$n" -gt 100 ] && exit 1
  /bin/sleep 0.2
done
/bin/rm -rf -- "$2" "$3" "$4"
/usr/bin/defaults delete "$5" >/dev/null 2>&1
/usr/bin/tccutil reset All "$5" >/dev/null 2>&1
[ -n "$6" ] && /bin/mv -- "$6" "$7"
exit 0
"#;

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub identifier: &'static str,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub models: PathBuf,
    /// The installed application, when it sits in an Applications folder; a copy run
    /// from a disk image or elsewhere is left alone.
    pub bundle: Option<PathBuf>,
    pub trash: PathBuf,
}

fn plain(path: &Path) -> bool {
    path.is_absolute() && path.components().all(|part| !matches!(part, Component::ParentDir | Component::CurDir))
}

/// Only the application's own per-identifier folders inside the user's library may be removed.
/// `product` is the configured application name, which names the installed bundle.
pub fn plan(identifier: &str, product: &str, config: &Path, cache: &Path, executable: &Path, trash: &Path) -> Result<Plan, &'static str> {
    let identifier = Profile::from_identifier(identifier)?.identifier();
    let bundle_name = format!("{product}.app");
    let owned = |path: &Path, parent: &str| {
        plain(path)
            && path.file_name().is_some_and(|name| name == identifier)
            && path.parent().and_then(Path::file_name).is_some_and(|name| name == parent)
            && path.parent().and_then(Path::parent).and_then(Path::file_name)
                .is_some_and(|name| name == "Library")
    };
    if !owned(config, "Application Support") || !owned(cache, "Caches")
        || !plain(trash) || trash.file_name().is_none_or(|name| name != ".Trash") {
        return Err("erase_invalid_location");
    }
    // <Applications>/<name>.app/Contents/MacOS/<executable>
    let bundle = executable.ancestors().nth(3).filter(|path| {
        plain(path)
            && path.extension().is_some_and(|extension| extension == "app")
            && path.file_name().is_some_and(|name| name == bundle_name.as_str())
            && path.parent().and_then(Path::file_name).is_some_and(|name| name == "Applications")
    });
    let models = config.parent().ok_or("erase_invalid_location")?.join(format!("{identifier}.models"));
    Ok(Plan {
        identifier,
        config: config.to_owned(),
        cache: cache.to_owned(),
        models,
        trash: bundle.map_or_else(|| trash.to_owned(), |_| trash.join(format!("{product} {}.app", unique()))),
        bundle: bundle.map(Path::to_owned),
    })
}

fn unique() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs())
}

#[cfg(target_os = "macos")]
fn schedule(plan: &Plan) -> Result<(), &'static str> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    Command::new("/bin/sh")
        .args(["-c", SCRIPT, "govorun-erase"])
        .arg(std::process::id().to_string())
        .arg(&plan.config)
        .arg(&plan.cache)
        .arg(&plan.models)
        .arg(plan.identifier)
        .arg(plan.bundle.as_deref().unwrap_or(Path::new("")))
        .arg(&plan.trash)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_| ())
        .map_err(|_| "erase_unavailable")
}

#[cfg(not(target_os = "macos"))]
fn schedule(_: &Plan) -> Result<(), &'static str> {
    Err("erase_unavailable")
}

#[tauri::command]
pub async fn erase_local_data(app: AppHandle, window: WebviewWindow, confirmed: bool) -> Result<(), &'static str> {
    if !windows::trusted(&window) { return Err("invalid_origin"); }
    if !confirmed { return Err("erase_unconfirmed"); }
    let plan = plan(
        &app.config().identifier,
        app.config().product_name.as_deref().unwrap_or_default(),
        &app.path().app_config_dir().map_err(|_| "erase_unavailable")?,
        &app.path().app_cache_dir().map_err(|_| "erase_unavailable")?,
        &std::env::current_exe().map_err(|_| "erase_unavailable")?,
        &app.path().home_dir().map_err(|_| "erase_unavailable")?.join(".Trash"),
    )?;
    #[cfg(target_os = "macos")]
    {
        // Launch-at-login would otherwise keep a registration for an empty installation.
        let _ = app.run_on_main_thread(|| { let _ = govorun_os_runtime::setup::set_login(false); });
    }
    schedule(&plan)?;
    app.exit(0);
    Ok(())
}
