//! Current Android option/profile codecs; selection and dictation decisions use
//! the same library helpers as desktop. No storage rewrite or application upload.
use super::{choices,stored,dictation,Options};
use crate::{Error,Result};
use serde_json::{json,Value};
fn valid_style(v:&str)->bool {v=="off" || matches!(v,"formal"|"light"|"concise") || v.starts_with("custom:")}
fn bound_profile(value:&Value)->Result<Value> {
    if !value.is_object() {return Ok(Value::Null)}
    let mut result=json!({});
    if let Some(v)=value["styleValue"].as_str().filter(|v|valid_style(v)) {result["styleValue"]=json!(v)}
    for key in ["emojiEnabled","reducedPunctuationEnabled"] {
        if !value[key].is_null() {result[key]=json!(value[key].as_bool().ok_or(Error::StorageInvalid)?)}
    }
    if let Some(v)=value["emojiMode"].as_str().filter(|v|!v.trim().is_empty()) {
        result["emojiMode"]=json!(if matches!(v,"off"|"restrained"|"lively"|"bright") {v} else {"off"});
    }
    Ok(if result.as_object().is_some_and(|v|v.is_empty()) {Value::Null} else {result})
}
fn profiles(value:&Value)->Result<Value> {
    let mut out=json!({});
    for (key,value) in value.as_object().ok_or(Error::StorageInvalid)? {
        let value=bound_profile(value)?;if !value.is_null() {out[key]=value}
    }
    Ok(out)
}
fn context(command:&Value)->Result<choices::Context> {
    let state=&command["state"];
    let wire=json!({"presets":state["presets"],"activeId":state["activeId"],"selectedStyle":state["selectedStyle"],
        "instruction":state["instruction"],"optionsByStyle":command["options"],"legacyOptions":{}});
    let reply=stored::read(serde_json::from_value(wire).map_err(|_|Error::StorageInvalid)?)?;
    let mut context:choices::Context=serde_json::from_value(reply["context"].clone()).map_err(|_|Error::StorageInvalid)?;
    if choices::selection(&context.state).is_none() {context.state.selected_style="formal".into()}
    Ok(context)
}
pub(crate) fn evaluate(command:Value)->Result<Value> {
    Ok(match command["kind"].as_str().ok_or(Error::InvalidRequest)? {
        "profiles"=>json!({"profiles":profiles(&command["profiles"])?}),
        "profile"=>{
            let value=bound_profile(&command["profile"])?;
            let base:Options=serde_json::from_value(command["base"].clone()).map_err(|_|Error::InvalidRequest)?;
            let context:choices::Context=serde_json::from_value(json!({"state":{"presets":[],"activeId":null,"selectedStyle":"formal","instruction":""},
                "builtInOptions":{"formal":base},"currentOptions":base,"currentOptionsStyle":"formal"})).map_err(|_|Error::InvalidRequest)?;
            let rule=rule(&value)?; let result=dictation::resolve_rule(context,rule.as_ref())?;
            json!({"profile":value,"inherited":value.is_null(),"valid":command["profile"]["styleValue"].is_null()
                || command["profile"]["styleValue"].as_str().is_some_and(valid_style),"options":result.payload})
        },
        "update"=>{
            let mut values=profiles(&command["profiles"])?;let key=command["package"].as_str().ok_or(Error::InvalidRequest)?;
            let value=bound_profile(&command["profile"])?;
            let valid=command["profile"]["styleValue"].is_null() || command["profile"]["styleValue"].as_str().is_some_and(valid_style);
            if value.is_null() || !valid {values.as_object_mut().ok_or(Error::StorageInvalid)?.remove(key);} else {values[key]=value;}
            json!({"profiles":values})
        },
        "snapshot"|"options"|"resolve"=>{
            let context=context(&command)?;
            let selected=choices::selection(&context.state).unwrap_or("formal".into());
            if command["kind"]=="snapshot" {json!({"selected":selected})}
            else if command["kind"]=="options" {
                let value=command["value"].as_str().ok_or(Error::InvalidRequest)?;
                let value=if !valid_style(value) {"formal"} else {value};
                json!({"options":choices::option_for(&context,value).unwrap_or_else(Options::defaults)})
            } else {
                let values=profiles(&command["profiles"])?;
                let value=command["package"].as_str().and_then(|key|values.get(key)).unwrap_or(&Value::Null);
                let mut bound=rule(value)?;
                if let Some(rule)=&mut bound {
                    if rule.style_value.as_deref().is_some_and(|v|v.starts_with("custom:") && choices::option_for(&context,v).is_none()) {
                        rule.style_value=Some("formal".into());
                    }
                }
                let resolved=dictation::resolve_rule(context.clone(),bound.as_ref())?;
                let id=bound.as_ref().and_then(|r|r.style_value.as_ref()).unwrap_or(&selected);
                let preset=id.strip_prefix("custom:").and_then(|id|context.state.presets.iter().find(|p|p.id==id));
                json!({"enabled":resolved.enabled,"options":resolved.payload,"preset":preset})
            }
        },
        _=>return Err(Error::InvalidRequest),
    })
}
fn rule(v:&Value)->Result<Option<choices::ApplicationRule>> {
    if v.is_null() {return Ok(None)}
    serde_json::from_value(json!({"applicationId":"local","applicationName":"local","styleValue":v["styleValue"],
        "emojiEnabled":v["emojiEnabled"],"emojiMode":v["emojiMode"],"reducedPunctuationEnabled":v["reducedPunctuationEnabled"]}))
        .map(Some).map_err(|_|Error::StorageInvalid)
}
