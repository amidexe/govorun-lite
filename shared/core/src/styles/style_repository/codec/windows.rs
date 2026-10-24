use super::{call, parse, usable, Error, Logical};
use crate::{
    style_definition::usable_stored,
    style_library::{choices::BUILT_INS, usable_application_rule},
};
use serde_json::{json, Value};
const EMOJI: [&str; 4] = ["off", "restrained", "lively", "bright"];
pub(super) fn root(raw: &Value) -> Result<Value, Error> {
    let fresh = json!({"inputModeContractVersion":1,"emojiMode":2,"emojiEnabled":true,"profanityFilterEnabled":true});
    let Some(mut root) = parse(raw)? else {
        return Ok(fresh);
    };
    let contract = field(&root, "inputModeContractVersion");
    if !root.is_object() || !(contract.is_null() || contract == &json!(1)) {
        return Err(Error::InvalidStorage);
    }
    // Hotkeys, recording, clipboard, own server and sign-out write this file too and may
    // create it first; until styles write theirs, style fields read as on a fresh install.
    if field(&root, "emojiMode").is_null() {
        for key in ["emojiMode", "emojiEnabled", "profanityFilterEnabled"] {
            if field(&root, key).is_null() {
                put(&mut root, key, fresh[key].clone());
            }
        }
    }
    Ok(root)
}
/// Each setting is read on its own: a value this version does not know (written by
/// another version or device) falls back to its default for that setting only and
/// never makes every style unreadable. Writes stay exact in `encoded_options`.
pub(super) fn read(raw: &Value) -> Result<Logical, Error> {
    let root = root(raw)?;
    let stored = field(&root, "selectedStyle");
    let selected = stored
        .as_str()
        .filter(|style| *style == "custom" || BUILT_INS.contains(style))
        .unwrap_or("formal");
    let mut skipped = !stored.is_null() && stored != selected;
    let id = field(&root, "selectedCustomStyleId")
        .as_str()
        .filter(|id| !id.is_empty());
    let presets = match field(&root, "customStyles") {
        Value::Array(values) => {
            let (kept, unread) = usable(values, preset, usable_stored, "id", usize::MAX);
            skipped |= !unread.is_empty() || values.iter().any(|value| options(field(value, "options")).1);
            kept
        }
        Value::Null => vec![],
        _ => return Err(Error::InvalidStorage),
    };
    let instruction = presets
        .iter()
        .find(|preset| preset["id"].as_str() == id)
        .map(|preset| preset["instruction"].clone());
    // A chosen custom style this version cannot read leaves the default style chosen;
    // dictation never sends a custom style without its instruction.
    let selected = match instruction {
        None if selected == "custom" => {
            skipped = true;
            "formal"
        }
        _ => selected,
    };
    let instruction = instruction.unwrap_or(json!(""));
    let mut built_in = serde_json::Map::new();
    if let Some(values) = field(&root, "builtInStyleOptions").as_object() {
        // A built-in style this version does not know stays in the file for its version.
        for (id, value) in values {
            let (read, unread) = options(value);
            skipped |= unread || !BUILT_INS.contains(&id.as_str());
            if BUILT_INS.contains(&id.as_str()) {
                built_in.insert(id.clone(), read);
            }
        }
    } else if !field(&root, "builtInStyleOptions").is_null() {
        return Err(Error::InvalidStorage);
    }
    let (current, unread) = options(&root);
    skipped |= unread;
    let context = json!({"state":{"presets":presets,"activeId":id,"selectedStyle":selected,"instruction":instruction},
        "builtInOptions":built_in,"currentOptions":current,"currentOptionsStyle":selected});
    call(json!({"kind":"choices","context":context})).map_err(|_| Error::InvalidStorage)?;
    let rules = match field(&root, "applicationStyleRules") {
        Value::Array(values) => {
            let (kept, unread) = usable(values, rule, usable_application_rule, "applicationId", 100);
            skipped |= !unread.is_empty() || values.iter().any(|value| {
                let mode = field(value, "emojiMode");
                !mode.is_null() && emoji(mode).is_none()
            });
            kept
        }
        Value::Null => vec![],
        _ => return Err(Error::InvalidStorage),
    };
    Ok(Logical {
        context,
        rules: json!(rules),
        options_present: true,
        skipped,
    })
}
fn preset(value: &Value) -> Value {
    let (options, _) = options(field(value, "options"));
    json!({"id":field(value,"id"),"name":field(value,"name"),"instruction":field(value,"instruction"),
        "punctuationMode":options["punctuationMode"],"emojiMode":options["emojiMode"],
        "emojiEnabled":options["emojiMode"]!="off","profanityFilterEnabled":options["profanityFilterEnabled"]})
}
fn rule(value: &Value) -> Value {
    // An unknown mode leaves the rule on its style's own emoji setting.
    let mode = emoji(field(value, "emojiMode")).map_or(Value::Null, |mode| json!(mode));
    json!({"applicationId":field(value,"applicationId"),"applicationName":field(value,"applicationName"),
        "styleValue":field(value,"styleValue"),"emojiEnabled":field(value,"emojiEnabled"),"emojiMode":mode,
        "reducedPunctuationEnabled":field(value,"reducedPunctuationEnabled")})
}
fn skipped(root: &Value, key: &str, view: fn(&Value) -> Value, admitted: fn(&Value) -> bool, id: &str, limit: usize) -> Vec<Value> {
    field(root, key)
        .as_array()
        .map_or_else(Vec::new, |values| usable(values, view, admitted, id, limit).1)
}
pub(super) fn write(
    raw: &mut Value,
    original: &Logical,
    context: &Value,
    rules: &Value,
) -> Result<(), Error> {
    let mut root = root(raw)?;
    let state = &context["state"];
    if state != &original.context["state"] {
        put(&mut root, "selectedStyle", state["selectedStyle"].clone());
        put(
            &mut root,
            "selectedCustomStyleId",
            state["activeId"]
                .as_str()
                .map(|id| json!(id))
                .unwrap_or(json!("")),
        );
        let mut presets=state["presets"].as_array().ok_or(Error::InvalidStorage)?.iter().map(|preset|
            Ok(json!({"id":preset["id"],"name":preset["name"],"instruction":preset["instruction"],"options":encoded_options(preset)?})))
            .collect::<Result<Vec<_>,Error>>()?;
        presets.extend(skipped(&root, "customStyles", preset, usable_stored, "id", usize::MAX));
        put(&mut root, "customStyles", json!(presets));
    }
    if context["builtInOptions"] != original.context["builtInOptions"] {
        let mut map: serde_json::Map<_, _> = field(&root, "builtInStyleOptions")
            .as_object()
            .map(|stored| stored.iter().filter(|(id, _)| !BUILT_INS.contains(&id.as_str())))
            .into_iter()
            .flatten()
            .map(|(id, options)| (id.clone(), options.clone()))
            .collect();
        for (id, options) in context["builtInOptions"]
            .as_object()
            .ok_or(Error::InvalidStorage)?
        {
            map.insert(id.clone(), encoded_options(options)?);
        }
        put(&mut root, "builtInStyleOptions", json!(map));
    }
    if context["currentOptions"] != original.context["currentOptions"] {
        for (field, value) in encoded_options(&context["currentOptions"])?
            .as_object()
            .ok_or(Error::InvalidStorage)?
        {
            put(&mut root, field, value.clone());
        }
    }
    if rules != &original.rules {
        let mut stored = rules.clone();
        let values = stored.as_array_mut().ok_or(Error::InvalidStorage)?;
        for item in values.iter_mut() {
            if !item["emojiMode"].is_null() {
                item["emojiMode"] = json!(emoji_number(&item["emojiMode"])?);
            }
        }
        values.extend(skipped(&root, "applicationStyleRules", rule, usable_application_rule, "applicationId", 100));
        put(&mut root, "applicationStyleRules", stored);
    }
    *raw = json!(root.to_string());
    Ok(())
}
pub(super) fn field<'a>(root: &'a Value, key: &str) -> &'a Value {
    root.as_object()
        .and_then(|object| {
            object
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(key))
        })
        .map(|(_, value)| value)
        .unwrap_or(&Value::Null)
}
pub(super) fn put(root: &mut Value, key: &str, value: Value) {
    let name = root
        .as_object()
        .and_then(|object| object.keys().find(|name| name.eq_ignore_ascii_case(key)))
        .cloned()
        .unwrap_or_else(|| key.into());
    root[name] = value;
}
fn emoji(value: &Value) -> Option<&'static str> {
    value
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| EMOJI.get(index))
        .copied()
}
fn emoji_number(value: &Value) -> Result<usize, Error> {
    EMOJI
        .iter()
        .position(|mode| Some(*mode) == value.as_str())
        .ok_or(Error::InvalidStorage)
}
/// The options and whether a stored value had to fall back to its default.
fn options(value: &Value) -> (Value, bool) {
    let (punctuation, mode) = (field(value, "punctuation"), field(value, "emojiMode"));
    let flag = |key| field(value, key).as_bool().unwrap_or(false);
    let unread = !(value.is_null() || value.is_object())
        || !(punctuation.is_null() || matches!(punctuation.as_u64(), Some(0 | 1)))
        || !(mode.is_null() || emoji(mode).is_some())
        || ["emojiEnabled", "profanityFilterEnabled"]
            .iter()
            .any(|key| !(field(value, key).is_null() || field(value, key).is_boolean()));
    let mut emoji_mode = emoji(mode).unwrap_or("off");
    if emoji_mode == "off" && flag("emojiEnabled") {
        emoji_mode = "bright";
    }
    let punctuation = if punctuation.as_u64() == Some(1) { "less" } else { "normal" };
    (json!({"punctuationMode":punctuation,"emojiMode":emoji_mode,"profanityFilterEnabled":flag("profanityFilterEnabled")}), unread)
}
fn encoded_options(value: &Value) -> Result<Value, Error> {
    let punctuation = match value["punctuationMode"].as_str() {
        Some("normal") => 0,
        Some("less") => 1,
        _ => return Err(Error::InvalidStorage),
    };
    Ok(
        json!({"punctuation":punctuation,"emojiMode":emoji_number(&value["emojiMode"])?,
        "emojiEnabled":value["emojiMode"]!="off","profanityFilterEnabled":value["profanityFilterEnabled"]}),
    )
}
