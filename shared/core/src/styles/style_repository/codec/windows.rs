use super::{call, parse, Error, Logical};
use serde_json::{json, Value};
const EMOJI: [&str; 4] = ["off", "restrained", "lively", "bright"];
pub(super) fn root(raw: &Value) -> Result<Value, Error> {
    let root=parse(raw)?.unwrap_or(json!({"inputModeContractVersion":1,"emojiMode":2,"emojiEnabled":true,"profanityFilterEnabled":true}));
    if !root.is_object()
        || field(&root, "inputModeContractVersion") != &json!(1)
        || field(&root, "emojiMode").is_null()
    {
        return Err(Error::InvalidStorage);
    }
    Ok(root)
}
pub(super) fn read(raw: &Value) -> Result<Logical, Error> {
    let root = root(raw)?;
    for key in ["selectedStyle", "selectedCustomStyleId"] {
        if present(&root, key) && !field(&root, key).is_string() {
            return Err(Error::InvalidStorage);
        }
    }
    let selected = field(&root, "selectedStyle").as_str().unwrap_or("formal");
    if !matches!(selected, "formal" | "light" | "concise" | "custom") {
        return Err(Error::InvalidStorage);
    }
    let id = field(&root, "selectedCustomStyleId")
        .as_str()
        .filter(|id| !id.is_empty());
    let mut presets = Vec::new();
    if let Some(values) = field(&root, "customStyles").as_array() {
        for preset in values {
            let options = options(field(preset, "options"))?;
            presets.push(json!({"id":field(preset,"id"),"name":field(preset,"name"),"instruction":field(preset,"instruction"),
                "punctuationMode":options["punctuationMode"],"emojiMode":options["emojiMode"],
                "emojiEnabled":options["emojiMode"]!="off","profanityFilterEnabled":options["profanityFilterEnabled"]}));
        }
    } else if present(&root, "customStyles") {
        return Err(Error::InvalidStorage);
    }
    let instruction = presets
        .iter()
        .find(|preset| preset["id"].as_str() == id)
        .map(|preset| preset["instruction"].clone())
        .unwrap_or(json!(""));
    let mut built_in = serde_json::Map::new();
    if let Some(values) = field(&root, "builtInStyleOptions").as_object() {
        for (id, value) in values {
            built_in.insert(id.clone(), options(value)?);
        }
    } else if present(&root, "builtInStyleOptions") {
        return Err(Error::InvalidStorage);
    }
    let context = json!({"state":{"presets":presets,"activeId":id,"selectedStyle":selected,"instruction":instruction},
        "builtInOptions":built_in,"currentOptions":options(&root)?,"currentOptionsStyle":selected});
    call(json!({"kind":"choices","context":context})).map_err(|_| Error::InvalidStorage)?;
    let mut rules = field(&root, "applicationStyleRules").clone();
    if rules.is_null() {
        rules = json!([]);
    }
    for rule in rules.as_array_mut().ok_or(Error::InvalidStorage)? {
        let mode = field(rule, "emojiMode");
        let mode = if mode.is_null() {
            Value::Null
        } else {
            json!(emoji(mode)?)
        };
        *rule = json!({"applicationId":field(rule,"applicationId"),"applicationName":field(rule,"applicationName"),
            "styleValue":field(rule,"styleValue"),"emojiEnabled":field(rule,"emojiEnabled"),"emojiMode":mode,
            "reducedPunctuationEnabled":field(rule,"reducedPunctuationEnabled")});
    }
    Ok(Logical {
        context,
        rules,
        options_present: true,
    })
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
        let presets=state["presets"].as_array().ok_or(Error::InvalidStorage)?.iter().map(|preset|
            Ok(json!({"id":preset["id"],"name":preset["name"],"instruction":preset["instruction"],"options":encoded_options(preset)?})))
            .collect::<Result<Vec<_>,Error>>()?;
        put(&mut root, "customStyles", json!(presets));
    }
    if context["builtInOptions"] != original.context["builtInOptions"] {
        let mut map = serde_json::Map::new();
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
        for rule in stored.as_array_mut().ok_or(Error::InvalidStorage)? {
            if !rule["emojiMode"].is_null() {
                rule["emojiMode"] = json!(emoji_number(&rule["emojiMode"])?);
            }
        }
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
fn present(root: &Value, key: &str) -> bool {
    root.as_object()
        .is_some_and(|object| object.keys().any(|name| name.eq_ignore_ascii_case(key)))
}
pub(super) fn put(root: &mut Value, key: &str, value: Value) {
    let name = root
        .as_object()
        .and_then(|object| object.keys().find(|name| name.eq_ignore_ascii_case(key)))
        .cloned()
        .unwrap_or_else(|| key.into());
    root[name] = value;
}
fn emoji(value: &Value) -> Result<&'static str, Error> {
    value
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| EMOJI.get(index))
        .copied()
        .ok_or(Error::InvalidStorage)
}
fn emoji_number(value: &Value) -> Result<usize, Error> {
    EMOJI
        .iter()
        .position(|mode| Some(*mode) == value.as_str())
        .ok_or(Error::InvalidStorage)
}
fn options(value: &Value) -> Result<Value, Error> {
    if !value.is_object() {
        return Err(Error::InvalidStorage);
    }
    let raw = field(value, "punctuation");
    let punctuation = match if !present(value, "punctuation") {
        0
    } else {
        raw.as_u64().ok_or(Error::InvalidStorage)?
    } {
        0 => "normal",
        1 => "less",
        _ => return Err(Error::InvalidStorage),
    };
    let mut mode = if !present(value, "emojiMode") {
        "off"
    } else {
        emoji(field(value, "emojiMode"))?
    };
    let enabled = boolean(value, "emojiEnabled")?;
    if mode == "off" && enabled {
        mode = "bright";
    }
    let profanity = boolean(value, "profanityFilterEnabled")?;
    Ok(json!({"punctuationMode":punctuation,"emojiMode":mode,"profanityFilterEnabled":profanity}))
}
fn boolean(value: &Value, key: &str) -> Result<bool, Error> {
    let missing = !present(value, key);
    let value = field(value, key);
    if missing {
        Ok(false)
    } else {
        value.as_bool().ok_or(Error::InvalidStorage)
    }
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
