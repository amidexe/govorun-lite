use super::{call, parse, Error, Logical};
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
    let reply=call(json!({"kind":"readStored","stored":{
        "presets":parse(&values[CATALOG])?,"activeId":values["govorunGatewayActiveCustomPresetV1"],
        "selectedStyle":values["govorunGatewayStyle"],"instruction":values.get("govorunGatewayCustomInstruction").unwrap_or(&json!("")),
        "optionsByStyle":options,"legacyOptions":stored_options(values,"")}})).map_err(|_|Error::InvalidStorage)?;
    let mut rules = parse(&values[RULES])?.unwrap_or(json!([]));
    for rule in rules.as_array_mut().ok_or(Error::InvalidStorage)? {
        let id = rule["bundleID"]
            .as_str()
            .ok_or(Error::InvalidStorage)?
            .to_owned();
        let name = rule["applicationName"]
            .as_str()
            .ok_or(Error::InvalidStorage)?
            .to_owned();
        *rule = json!({"applicationId":id,"applicationName":name,"styleValue":rule["styleValue"],
            "emojiEnabled":rule["emojiEnabled"],"emojiMode":rule["emojiMode"],
            "reducedPunctuationEnabled":rule["reducedPunctuationEnabled"]});
    }
    Ok(Logical {
        context: reply["context"].clone(),
        rules,
        options_present: reply["storedOptionsPresent"]
            .as_bool()
            .ok_or(Error::InvalidStorage)?,
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
        values[CATALOG] = json!(state["presets"].to_string());
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
        values[RULES] = json!(stored.to_string());
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
