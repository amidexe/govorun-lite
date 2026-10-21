//! Protected wire boundary over the existing DPAPI slots; no account or billing rules.
use super::{
    file_documents::{Disk, WRITER},
    secure_files::{self, Error},
};
use serde_json::{json, Value};

pub trait Protected: Send + Sync {
    fn read(&self, key: &str) -> Result<Option<String>, Error>;
    fn write(&self, key: &str, raw: &str) -> Result<(), Error>;
    fn delete(&self, key: &str) -> Result<(), Error>;
    fn compare_write(&self, key: &str, expected: Option<&str>, raw: &str) -> Result<String, Error>;
    fn restart(&self) -> Result<(), Error>;
}
#[cfg(target_os = "windows")]
impl Protected for secure_files::Store<super::windows_dpapi::Dpapi> {
    fn read(&self, key: &str) -> Result<Option<String>, Error> {
        self.read(key)
    }
    fn write(&self, key: &str, raw: &str) -> Result<(), Error> {
        self.write(key, raw)
    }
    fn delete(&self, key: &str) -> Result<(), Error> {
        self.delete(key)
    }
    fn compare_write(&self, key: &str, expected: Option<&str>, raw: &str) -> Result<String, Error> {
        self.compare_write(key, expected, raw)
    }
    fn restart(&self) -> Result<(), Error> {
        self.restart(&uuid::Uuid::new_v4().simple().to_string())
    }
}
pub fn handles(key: &str) -> bool {
    matches!(
        key,
        "account.session"
            | "account.authAttempt"
            | "stylePass.vault"
            | "selfHost.ollama"
            | "selfHost.openai"
            | "selfHost.gemini"
    ) || secure_files::checkout_key(key)
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|fields| {
        fields.len() == keys.len() && keys.iter().all(|key| fields.contains_key(*key))
    })
}
fn parse(raw: &str, maximum: usize) -> Result<Value, Error> {
    if raw.len() > maximum {
        return Err(Error::Invalid);
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
    if !value.is_object() && !value.is_null() {
        return Err(Error::Invalid);
    }
    Ok(value)
}
fn attempt(raw: &str) -> Result<Value, Error> {
    if raw.encode_utf16().count() > 8192 {
        return Err(Error::Invalid);
    }
    parse(raw, 8192 * 4)
}
fn auth_load(store: &impl Protected) -> Result<Value, Error> {
    if let Some(raw) = store.read("account-auth-attempt-v1")? {
        let value = attempt(&raw)?;
        return Ok(if value.is_null() {
            json!({"status":"missing"})
        } else {
            json!({"status":"available","value":value})
        });
    }
    Ok(json!({"status":"missing"}))
}
fn auth_save(store: &impl Protected, value: &Value) -> Result<Value, Error> {
    let raw = value.to_string();
    attempt(&raw)?;
    store.write("account-auth-attempt-v1", &raw)?;
    Ok(json!({"status":"available"}))
}
fn settings(disk: &impl Disk) -> Result<Value, Error> {
    let raw = disk
        .read("settings.json", 8 * 1_048_576)
        .map_err(|_| Error::Unavailable)?;
    let Some(raw) = raw else {
        return Ok(json!({}));
    };
    let normalized = raw.trim_start_matches('\u{feff}');
    super::file_documents::settings_document::validate(
        "styles.document",
        Some(normalized),
        normalized,
    )
    .map_err(|_| Error::Unavailable)?;
    let value =
        parse(raw.trim_start_matches('\u{feff}'), 8 * 1_048_576).map_err(|_| Error::Unavailable)?;
    if value["inputModeContractVersion"] != 1
        || value["emojiMode"]
            .as_i64()
            // The style codec writes four emoji modes (off, restrained, lively, bright = 0..=3);
            // «Ярко» made every session read unavailable here since 6 Oct 2026.
            .is_none_or(|n| !(0..=3).contains(&n))
    {
        return Err(Error::Unavailable);
    }
    let markers: Vec<_> = value
        .as_object()
        .ok_or(Error::Unavailable)?
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case("accountSessionEnded"))
        .collect();
    if markers.len() > 1 || markers.first().is_some_and(|(_, v)| !v.is_boolean()) {
        return Err(Error::Unavailable);
    }
    Ok(value)
}
fn ended(settings: &Value) -> bool {
    settings
        .as_object()
        .and_then(|fields| {
            fields
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("accountSessionEnded"))
        })
        .and_then(|(_, v)| v.as_bool())
        .unwrap_or(false)
}
fn marker(disk: &impl Disk, mut value: Value, ended: bool) -> Result<(), Error> {
    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        // Exact fresh-file format; preferences stay owned by the common documents.
        value = json!({"inputModeContractVersion":1,"emojiMode":2,"emojiEnabled":true,"profanityFilterEnabled":true});
    }
    let key = value
        .as_object()
        .and_then(|fields| {
            fields
                .keys()
                .find(|key| key.eq_ignore_ascii_case("accountSessionEnded"))
        })
        .cloned()
        .unwrap_or_else(|| "accountSessionEnded".into());
    value[&key] = json!(ended);
    let raw = value.to_string();
    disk.write("settings.json", &raw)
        .map_err(|_| Error::Unavailable)?;
    if disk
        .read("settings.json", 8 * 1_048_576)
        .map_err(|_| Error::Unavailable)?
        .as_deref()
        != Some(&raw)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
fn session(value: &Value) -> Result<(), Error> {
    if value["version"] != 1 || !value["tokens"].is_object() {
        return Err(Error::Invalid);
    }
    for field in ["accessToken", "refreshToken"] {
        if value["tokens"][field]
            .as_str()
            .is_none_or(|s| s.trim().is_empty() || s == "host-owned-session")
        {
            return Err(Error::Invalid);
        }
    }
    for field in ["displayName", "deviceId"] {
        if value["tokens"]
            .get(field)
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            return Err(Error::Invalid);
        }
    }
    if value["tokens"]
        .get("deviceLimit")
        .is_some_and(|v| v.as_i64().is_none_or(|n| i32::try_from(n).is_err()))
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn expected(params: &Value) -> Result<Option<&str>, Error> {
    let value = params.get("expectedRevision").ok_or(Error::Invalid)?;
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .map(Some)
        .ok_or(Error::Invalid)
}
fn execute(
    store: &impl Protected,
    disk: &impl Disk,
    method: &str,
    params: &Value,
) -> Result<Value, Error> {
    let _guard = WRITER.lock().map_err(|_| Error::Unavailable)?;
    let key = params["key"].as_str().ok_or(Error::Invalid)?;
    if secure_files::checkout_key(key) {
        if !matches!(method, "storage.read" | "storage.write" | "storage.delete") {
            return Err(Error::Invalid);
        }
        let raw = store.read(key)?;
        let value = raw.as_deref().map(|raw| parse(raw, 65_536)).transpose()?;
        let value = value.filter(|v| !v.is_null());
        let revision = value
            .as_ref()
            .and(raw.as_deref())
            .map(secure_files::revision);
        if method == "storage.read" {
            return Ok(match value {
                None => json!({"status":"missing"}),
                Some(value) => json!({"status":"available","revision":revision,"value":value}),
            });
        }
        if expected(params)? != revision.as_deref() {
            return Err(Error::Conflict);
        }
        let next = if method == "storage.delete" {
            Value::Null
        } else {
            params
                .get("value")
                .filter(|v| v.is_object())
                .ok_or(Error::Invalid)?
                .clone()
        };
        let raw = next.to_string();
        parse(&raw, 65_536)?;
        store.write(key, &raw)?;
        return Ok(if method == "storage.delete" {
            json!({"status":"available"})
        } else {
            json!({"status":"available","revision":secure_files::revision(&raw),"value":next})
        });
    }
    match key {
        "account.authAttempt" => match method {
            "secret.read" => auth_load(store),
            "secret.write" if params["value"].is_object() => auth_save(store, &params["value"]),
            "secret.delete" => auth_save(store, &Value::Null),
            _ => Err(Error::Invalid),
        },
        "account.session" => {
            let current = settings(disk)?;
            match method {
                "secret.read" => {
                    if ended(&current) {
                        return Ok(json!({"status":"missing"}));
                    }
                    let Some(raw) = store.read("account-session-v2")? else {
                        return Ok(json!({"status":"missing"}));
                    };
                    let value = parse(&raw, 9 * 1_048_576)?;
                    if value["version"] != 1 || !value["tokens"].is_object() {
                        return Ok(json!({"status":"missing"}));
                    }
                    session(&value)?;
                    Ok(json!({"status":"available","value":value}))
                }
                "secret.write" => {
                    let value = &params["value"];
                    session(value)?;
                    store.write("account-session-v2", &value.to_string())?;
                    if ended(&current) {
                        marker(disk, current, false)?;
                    }
                    Ok(json!({"status":"available"}))
                }
                "secret.delete" => {
                    auth_save(store, &Value::Null)?;
                    if !ended(&current) {
                        marker(disk, current, true)?;
                    }
                    let _ = store.delete("account-session-v2");
                    Ok(json!({"status":"available"}))
                }
                "secret.restart" => {
                    store.restart()?;
                    marker(disk, current, true)?;
                    Ok(json!({"status":"available"}))
                }
                _ => Err(Error::Invalid),
            }
        }
        "stylePass.vault" => {
            if !params.as_object().is_some_and(|p| {
                p.keys().all(|k| {
                    matches!(
                        k.as_str(),
                        "key" | "value" | "expectedRevision" | "allowInteraction"
                    )
                })
            }) || params
                .get("allowInteraction")
                .is_some_and(|v| !v.is_boolean())
            {
                return Err(Error::Invalid);
            }
            if method == "secret.read" {
                let Some(raw) = store.read("ready-pass-pool-v1")? else {
                    return Ok(json!({"status":"missing"}));
                };
                if raw.is_empty() || raw.len() > 4_194_304 {
                    return Err(Error::Invalid);
                }
                return Ok(
                    json!({"status":"available","value":raw,"revision":secure_files::revision(&raw)}),
                );
            }
            if method != "secret.write" {
                return Err(Error::Invalid);
            }
            let raw = params["value"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 4_194_304)
                .ok_or(Error::Invalid)?;
            let revision = store.compare_write("ready-pass-pool-v1", expected(params)?, raw)?;
            Ok(json!({"status":"available","revision":revision}))
        }
        "selfHost.ollama" | "selfHost.openai" | "selfHost.gemini" => {
            let slot = match key {
                "selfHost.ollama" => "custom-server-api-key",
                "selfHost.openai" => "custom-server-openai-key",
                _ => "custom-server-gemini-key",
            };
            match method {
                "secret.read" => {
                    let (state, key) = match store.read(slot) {
                        Ok(Some(raw)) if !raw.trim().is_empty() => ("saved", Some(raw)),
                        Ok(_) => ("missing", None),
                        Err(_) => ("inaccessible", None),
                    };
                    Ok(json!({"status":"available","value":{"state":state,"key":key}}))
                }
                "secret.write" if exact(&params["value"], &["key"]) => {
                    let raw = params["value"]["key"]
                        .as_str()
                        .filter(|s| s.len() <= 16_384)
                        .ok_or(Error::Invalid)?;
                    store.write(slot, raw)?;
                    Ok(json!({"status":"available"}))
                }
                "secret.delete" => {
                    store.delete(slot)?;
                    Ok(json!({"status":"available"}))
                }
                _ => Err(Error::Invalid),
            }
        }
        _ => Err(Error::Invalid),
    }
}
pub fn invoke(store: &impl Protected, disk: &impl Disk, method: &str, params: &Value) -> Value {
    execute(store, disk, method, params).unwrap_or_else(|e| json!({"status":match e { Error::Invalid=>"invalid", Error::Unavailable=>"unavailable", Error::Conflict=>"conflict"}}))
}
