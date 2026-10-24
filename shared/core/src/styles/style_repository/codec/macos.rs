use super::{call, parse, usable, Error, Logical};
use crate::style_library::{usable_application_rule, usable_stored_preset};
use serde_json::{json, Value};
const CATALOG: &str = "govorunGatewayCustomPresetsV1";
const RULES: &str = "govorunApplicationStyleRulesV1";
pub(super) fn read(values: &Value) -> Result<Logical, Error> {
    if !values.is_object() {
        return Err(Error::InvalidStorage);
    }
    let mut options = serde_json::Map::new();
    for style in ["formal", "light", "concise", "custom"] {
        options.insert(style.into(), stored_options(values, &format!(".{style}")));
    }
    // A preset or rule this version cannot use is skipped and stays stored; an
    // unreadable catalog reads as no presets and is never overwritten by a save.
    let mut skipped = false;
    let presets = match parse(&values[CATALOG]) {
        Ok(Some(Value::Array(items))) => {
            let (kept, unread) = usable(&items, Value::clone, usable_stored_preset, "id", 1000);
            skipped |= !unread.is_empty();
            json!(kept)
        }
        Ok(None) => Value::Null,
        _ => {
            skipped = true;
            Value::Null
        }
    };
    // A chosen preset this version skipped leaves the default style chosen; dictation
    // never sends a custom style without its instruction. The stored choice stays.
    let active = values["govorunGatewayActiveCustomPresetV1"].as_str();
    let mut selected = values["govorunGatewayStyle"].clone();
    if skipped && selected == "custom" && active.is_some()
        && !presets.as_array().is_some_and(|kept| kept.iter().any(|preset| preset["id"].as_str() == active)) {
        selected = json!("formal");
    }
    let reply=call(json!({"kind":"readStored","stored":{
        "presets":presets,"activeId":values["govorunGatewayActiveCustomPresetV1"],
        "selectedStyle":selected,"instruction":values.get("govorunGatewayCustomInstruction").unwrap_or(&json!("")),
        "optionsByStyle":options,"legacyOptions":stored_options(values,"")}})).map_err(|_|Error::InvalidStorage)?;
    let rules = match parse(&values[RULES]) {
        Ok(Some(Value::Array(items))) => {
            let (kept, unread) = usable(&items, rule, usable_application_rule, "applicationId", 100);
            skipped |= !unread.is_empty();
            kept
        }
        Ok(None) => vec![],
        _ => {
            skipped = true;
            vec![]
        }
    };
    Ok(Logical {
        context: reply["context"].clone(),
        rules: json!(rules),
        options_present: reply["storedOptionsPresent"]
            .as_bool()
            .ok_or(Error::InvalidStorage)?,
        skipped,
    })
}
pub(super) fn write(
    values: &mut Value,
    original: &Logical,
    context: &Value,
    rules: &Value,
) -> Result<(), Error> {
    let state = &context["state"];
    let previous = &original.context["state"];
    if state["presets"] != previous["presets"] {
        let mut presets = state["presets"].as_array().ok_or(Error::InvalidStorage)?.clone();
        presets.extend(unread(values, CATALOG, Value::clone, usable_stored_preset, "id", 1000)?);
        values[CATALOG] = json!(Value::Array(presets).to_string());
    }
    for (field, key) in [
        ("selectedStyle", "govorunGatewayStyle"),
        ("activeId", "govorunGatewayActiveCustomPresetV1"),
        ("instruction", "govorunGatewayCustomInstruction"),
    ] {
        if state[field] != previous[field] {
            if state[field].is_null() {
                values
                    .as_object_mut()
                    .ok_or(Error::InvalidStorage)?
                    .remove(key);
            } else {
                values[key] = state[field].clone();
            }
        }
    }
    if context["builtInOptions"] != original.context["builtInOptions"]
        || context["currentOptions"] != original.context["currentOptions"]
        || context["currentOptionsStyle"] != original.context["currentOptionsStyle"]
    {
        for (style, options) in context["builtInOptions"]
            .as_object()
            .ok_or(Error::InvalidStorage)?
        {
            put(values, &format!(".{style}"), options);
        }
        put(values, "", &context["currentOptions"]);
    }
    if rules != &original.rules {
        let mut stored = rules.clone();
        for rule in stored.as_array_mut().ok_or(Error::InvalidStorage)? {
            let object = rule.as_object_mut().ok_or(Error::InvalidStorage)?;
            let id = object
                .remove("applicationId")
                .ok_or(Error::InvalidStorage)?;
            object.insert("bundleID".into(), id);
        }
        let mut stored = stored.as_array().ok_or(Error::InvalidStorage)?.clone();
        stored.extend(unread(values, RULES, rule, usable_application_rule, "applicationId", 100)?);
        values[RULES] = json!(Value::Array(stored).to_string());
    }
    Ok(())
}
fn stored_options(values: &Value, suffix: &str) -> Value {
    json!({"punctuationMode":values[format!("govorunGatewayPunctuationMode{suffix}")],
        "reducedPunctuation":if suffix.is_empty() {values["govorunGatewayReducedPunctuation"].clone()} else {Value::Null},
        "emojiEnabled":values[format!("govorunGatewayEmojiEnabled{suffix}")],
        "emojiMode":values[format!("govorunGatewayEmojiMode{suffix}")],
        "profanityFilterEnabled":values[format!("govorunGatewayProfanityFilterEnabled{suffix}")]})
}
fn put(values: &mut Value, suffix: &str, options: &Value) {
    for (key, field) in [
        ("PunctuationMode", "punctuationMode"),
        ("EmojiMode", "emojiMode"),
        ("ProfanityFilterEnabled", "profanityFilterEnabled"),
    ] {
        values[format!("govorunGateway{key}{suffix}")] = options[field].clone();
    }
    values[format!("govorunGatewayEmojiEnabled{suffix}")] = json!(options["emojiMode"] != "off");
    if suffix.is_empty() {
        values["govorunGatewayReducedPunctuation"] = json!(options["punctuationMode"] == "less");
    }
}
fn rule(value: &Value) -> Value {
    json!({"applicationId":value["bundleID"],"applicationName":value["applicationName"],
        "styleValue":value["styleValue"],"emojiEnabled":value["emojiEnabled"],"emojiMode":value["emojiMode"],
        "reducedPunctuationEnabled":value["reducedPunctuationEnabled"]})
}
/// Stored entries skipped on read, kept as they are; an unreadable list is never overwritten.
fn unread(values: &Value, key: &str, view: fn(&Value) -> Value, admitted: fn(&Value) -> bool, id: &str, limit: usize)
    -> Result<Vec<Value>, Error> {
    Ok(match parse(&values[key])? {
        Some(Value::Array(items)) => usable(&items, view, admitted, id, limit).1,
        Some(_) => return Err(Error::InvalidStorage),
        None => vec![],
    })
}

