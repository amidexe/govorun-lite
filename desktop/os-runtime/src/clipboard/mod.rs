//! Intentional user copy; only supplied report text is written, nothing is read.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
pub const MAXIMUM: usize = 131_072;
pub fn copy(text: &str) -> Result<(), &'static str> {
    if text.len() > MAXIMUM {
        return Err("diagnostics_report_too_large");
    }
    #[cfg(target_os = "macos")]
    {
        macos::copy("NSGeneralPboard", text)
    }
    #[cfg(target_os = "windows")]
    {
        windows::copy(text)
    }
}
#[cfg(all(target_os = "macos", feature = "private-fixtures"))]
pub fn copy_named(name: &str, text: &str) -> Result<(), &'static str> {
    if !name.starts_with("govorun.synthetic.") || name.len() > 200 || text.len() > MAXIMUM {
        return Err("invalid_parameters");
    }
    macos::copy(name, text)
}
