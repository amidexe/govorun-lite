//! The existing settings.json byte boundary. Product validation stays in core.
use super::Error;
use serde_json::Value;
use std::sync::OnceLock;

fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../shared/core/contracts/storage-keys.json"))
            .expect("storage key table")
    })
}
pub fn handles(key: &str) -> bool {
    matches!(
        key,
        "styles.document" | "selfHost.document" | "hotkeys.document" | "recording.document"
    )
}
pub fn validate(key: &str, previous: Option<&str>, raw: &str) -> Result<(), Error> {
    // System.Text.Json DTO writes did not accept a leading BOM. Existing reads
    // remain opaque and preserve it; only replacement bytes are checked here.
    if raw.starts_with('\u{feff}') {
        return Err(Error::Invalid);
    }
    let before = match previous {
        Some(raw) => serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}'))
            .map_err(|_| Error::Invalid)?,
        None => serde_json::json!({}),
    };
    let after: Value =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).map_err(|_| Error::Invalid)?;
    let before = before.as_object().ok_or(Error::Invalid)?;
    let after = after.as_object().ok_or(Error::Invalid)?;
    let fields = table()["documents"][key]["windows"]["mutableFields"]
        .as_array()
        .ok_or(Error::Invalid)?;
    for name in before.keys().chain(after.keys()) {
        if fields.iter().any(|field| {
            field
                .as_str()
                .is_some_and(|field| field.eq_ignore_ascii_case(name))
        }) {
            continue;
        }
        if key == "styles.document"
            && previous.is_none()
            && name == "inputModeContractVersion"
            && after.get(name) == Some(&serde_json::json!(1))
        {
            continue;
        }
        if before.get(name) != after.get(name) {
            return Err(Error::Invalid);
        }
    }
    // Preserve the old DTO decoder's JSON type check, without its product policy
    // or a second mutable settings cache. Unknown fields remain opaque.
    if !after
        .iter()
        .all(|(name, value)| field_type(&name.to_ascii_lowercase(), value))
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
fn integer(value: &Value) -> bool {
    value.as_i64().is_some_and(|v| i32::try_from(v).is_ok())
}
fn nullable(value: &Value, predicate: fn(&Value) -> bool) -> bool {
    value.is_null() || predicate(value)
}
fn string(value: &Value) -> bool {
    value.is_string()
}
fn object(value: &Value, predicate: fn(&str, &Value) -> bool) -> bool {
    value.as_object().is_some_and(|fields| {
        fields
            .iter()
            .all(|(name, value)| predicate(&name.to_ascii_lowercase(), value))
    })
}
fn options(value: &Value) -> bool {
    object(value, |name, value| match name {
        "punctuation" | "emojimode" => integer(value),
        "emojienabled" | "profanityfilterenabled" => value.is_boolean(),
        _ => true,
    })
}
fn preset(value: &Value) -> bool {
    object(value, |name, value| match name {
        "id" | "name" | "instruction" => nullable(value, string),
        "options" => nullable(value, options),
        _ => true,
    })
}
fn rule(value: &Value) -> bool {
    object(value, |name, value| match name {
        "applicationid" | "applicationname" | "stylevalue" => nullable(value, string),
        "emojienabled" | "reducedpunctuationenabled" => nullable(value, Value::is_boolean),
        "emojimode" => nullable(value, integer),
        _ => true,
    })
}
fn provider(value: &Value) -> bool {
    object(value, |name, value| match name {
        "endpoint" | "model" => nullable(value, string),
        _ => true,
    })
}
fn dictionary(value: &Value, item: fn(&Value) -> bool) -> bool {
    value.is_null()
        || value
            .as_object()
            .is_some_and(|fields| fields.values().all(|value| nullable(value, item)))
}
fn list(value: &Value, item: fn(&Value) -> bool) -> bool {
    value.is_null()
        || value
            .as_array()
            .is_some_and(|items| items.iter().all(|value| nullable(value, item)))
}
fn field_type(name: &str, value: &Value) -> bool {
    match name {
        "accountsessionended"
        | "stylingenabled"
        | "emojienabled"
        | "profanityfilterenabled"
        | "starterguidancevisible"
        | "playsounds"
        | "pushtotalk"
        | "keeprecognitionmodelloaded"
        | "alwaysonmicrophone"
        | "muteaudioduringrecording"
        | "launchatlogin"
        | "hotkeysetupcompleted"
        | "customserverenabled"
        | "customserverstylingenabled"
        | "customserverproxyenabled" => value.is_boolean(),
        "acceptedonlinetermsversion"
        | "punctuation"
        | "emojimode"
        | "voicemarker"
        | "inputmodecontractversion"
        | "asracceleration"
        | "theme"
        | "phrasepausemilliseconds"
        | "maxrecordingminutes"
        | "customserverminimumcharacters" => integer(value),
        "audioinputchannel" => nullable(value, integer),
        "selectedstyle"
        | "selectedcustomstyleid"
        | "audioinputdevice"
        | "hotkey"
        | "plainhotkey"
        | "styledhotkey"
        | "cancelhotkey"
        | "customserverprovider"
        | "customserverstyle"
        | "customserverendpoint"
        | "customservermodel"
        | "customserverprompt"
        | "customserveracceptedrecipient"
        | "customserververifiedconfiguration"
        | "customserverproxyurl" => nullable(value, string),
        "builtinstyleoptions" => dictionary(value, options),
        "customstyles" | "customserverstyles" => list(value, preset),
        "applicationstylerules" => list(value, rule),
        "customserverproviders" => dictionary(value, provider),
        _ => true,
    }
}
