//! Existing OS login-item registration, with no settings file or migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Login {
    Enabled,
    Disabled,
    ApprovalRequired,
    Unavailable,
}
pub fn macos_status(value: isize) -> Login {
    match value {
        0 => Login::Disabled,
        1 => Login::Enabled,
        2 => Login::ApprovalRequired,
        _ => Login::Unavailable,
    }
}
pub fn command(executable: &str) -> Result<String, &'static str> {
    let (_, command) = commands(&executable.encode_utf16().collect::<Vec<_>>())?;
    String::from_utf16(&command).map_err(|_| "login_invalid_executable")
}
pub fn commands(path: &[u16]) -> Result<(Vec<u16>, Vec<u16>), &'static str> {
    if path.is_empty() || path.len() > 32760 || path.iter().any(|unit| *unit == 0 || *unit == 34) {
        return Err("login_invalid_executable");
    }
    let mut legacy = vec![34];
    legacy.extend_from_slice(path);
    legacy.push(34);
    let mut command = legacy.clone();
    command.extend(" --background".encode_utf16());
    Ok((legacy, command))
}
pub fn value_name(identifier: &str) -> Result<&'static str, &'static str> {
    match identifier {
        "com.govorun.app" => Ok("Govorun"),
        "com.govorun.app.testbackend" => Ok("Govorun.TestBackend"),
        _ => Err("login_unknown_profile"),
    }
}
