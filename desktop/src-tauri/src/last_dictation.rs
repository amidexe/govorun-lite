//! The last dictated text, kept only in process memory for «Скопировать последнюю диктовку».
//! Off by default; nothing is ever written to disk. The text is replaced by the next dictation,
//! dropped when the person turns the option off, and gone on quit. Follows Handy's tray action
//! (MIT, cjpais/Handy #598) without its on-disk history.
use serde_json::{json, Value};
use std::sync::Mutex;

struct Last {
    enabled: bool,
    text: Option<String>,
    // The local transcript when online improvement changed it, for a side-by-side comparison.
    original: Option<String>,
}
static LAST: Mutex<Last> = Mutex::new(Last { enabled: false, text: None, original: None });

/// Returns true when the setting changed, so the tray menu shows or hides its item.
pub fn set_enabled(enabled: bool) -> bool {
    let Ok(mut last) = LAST.lock() else { return false };
    if !enabled {
        last.text = None;
        last.original = None;
    }
    std::mem::replace(&mut last.enabled, enabled) != enabled
}

pub fn enabled() -> bool {
    LAST.lock().is_ok_and(|last| last.enabled)
}

pub fn remember(text: &str) {
    let text = text.trim();
    if let Ok(mut last) = LAST.lock() {
        if last.enabled && !text.is_empty() {
            last.text = Some(text.to_owned());
            last.original = None;
        }
    }
}

/// Called right after the improved text was remembered; an unchanged transcript adds nothing.
pub fn keep_original(original: &str) {
    let original = original.trim();
    if let Ok(mut last) = LAST.lock() {
        if last.enabled && !original.is_empty() && last.text.as_deref() != Some(original) {
            last.original = Some(original.to_owned());
        }
    }
}

/// The texts for the main window card: (pasted, original); None while off or before the first dictation.
pub fn texts() -> (Option<String>, Option<String>) {
    LAST.lock().map(|last| (last.text.clone(), last.original.clone())).unwrap_or_default()
}

/// An intentional user copy to the system clipboard; nothing happens before the first dictation.
pub fn copy(original: bool) -> Result<bool, &'static str> {
    let last = LAST.lock().map_err(|_| "last_dictation_unavailable")?;
    let text = if original { last.original.clone() } else { last.text.clone() };
    drop(last);
    match text {
        Some(text) => govorun_os_runtime::clipboard::copy(&text).map(|()| true),
        None => Ok(false),
    }
}

/// The main window's commands on the clipboard route; the settings document never sees the text.
pub fn invoke(method: &str, params: &Value) -> Option<Result<Value, &'static str>> {
    let fields = params.as_object();
    Some(match method {
        "lastDictation.snapshot" if fields.is_some_and(|value| value.is_empty()) => {
            let (text, original) = texts();
            Ok(json!({"text": text, "original": original}))
        }
        "lastDictation.copy" => match fields {
            Some(value) if value.is_empty() => copy(false),
            Some(value) if value.len() == 1 => match value.get("original").and_then(Value::as_bool) {
                Some(original) => copy(original),
                None => Err("invalid_command"),
            },
            _ => Err("invalid_command"),
        }
        .map(|copied| json!({"copied": copied})),
        "lastDictation.snapshot" => Err("invalid_command"),
        _ => return None,
    })
}
