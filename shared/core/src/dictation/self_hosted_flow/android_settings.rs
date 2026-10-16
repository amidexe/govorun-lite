//! Current Android preference/URI codecs. JVM parses URI syntax; Core owns
//! route admission, suffixes, stored hash tokens and self-owned catalog rules.
use crate::{Error,Result};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
fn text<'a>(v:&'a Value,k:&str)->&'a str {v[k].as_str().unwrap_or("")}
fn take(v:&str,max:usize)->String {let mut count=0;v.chars().take_while(|c|{count+=c.len_utf16();count<=max}).collect()}
fn fingerprint(endpoint:&str,model:&str)->Option<String> {
    (!endpoint.is_empty() && !model.trim().is_empty()).then(||format!("{:x}",Sha256::digest(format!("{endpoint}\n{}",model.trim()))))
}
pub(crate) fn evaluate(command:Value)->Result<Value> {
    Ok(match text(&command,"kind") {
        "address" => {
            let mut v=command["parsed"].clone(); let proxy=command["proxy"]==true;
            let scheme=text(&v,"scheme").to_lowercase();let host=text(&v,"host").to_owned();
            let port=v["port"].as_i64().ok_or(Error::InvalidRequest)?;
            let path=text(&v,"path");
            if host.is_empty() || !v["userInfo"].is_null() || !v["fragment"].is_null()
                || (if proxy {!matches!(scheme.as_str(),"http"|"socks5") || !v["query"].is_null()
                    || !matches!(path,""|"/") || !(port == -1 || (1..=65535).contains(&port))}
                    else {!matches!(scheme.as_str(),"http"|"https")}) {return Err(Error::InvalidRequest)}
            let path=if proxy {String::new()} else {super::endpoint::chat_path(path,true)};
            v["scheme"]=json!(scheme);v["path"]=json!(path);
            if proxy {v["host"]=json!(host.to_lowercase());v["port"]=json!(if port == -1 {if scheme=="socks5" {1080} else {8080}} else {port});}
            v
        },
        "fingerprint" => json!({"value":fingerprint(text(&command,"endpoint"),text(&command,"model"))}),
        "recipient" => {
            let token=fingerprint(text(&command,"endpoint"),"consent-v1");
            let proxy=command["proxy"]==true;
            json!({"value":token.filter(|_|!proxy || !text(&command,"proxyURL").is_empty())
                .map(|token|if proxy {format!("{token}\n{}",text(&command,"proxyURL"))} else {token})})
        },
        "settings" => {
            let v=&command["value"];let old=&command["previous"];
            let complete=!text(v,"endpoint").is_empty() && !text(v,"model").trim().is_empty()
                && text(v,"prompt").trim().encode_utf16().count()>=3
                && (v["proxyEnabled"]!=true || !text(v,"proxyURL").is_empty());
            let consent=!text(v,"accepted").is_empty() && v["accepted"]==v["recipient"];
            let key=take(text(v,"key").trim(),4096);let model=take(text(v,"model").trim(),120);
            json!({"complete":complete,"consent":consent,"model":model,"key":key,"prompt":text(v,"prompt").trim(),
                "accepted":if consent {text(v,"accepted")} else {""},
                "invalidate":fingerprint(text(old,"endpoint"),text(old,"model"))!=fingerprint(text(v,"endpoint"),&model)
                    || old["recipient"]!=v["recipient"] || text(old,"key")!=key})
        },
        "selection" => {
            let selected=command["selected"].as_str().unwrap_or("general");
            let selected=if selected=="general" || selected.starts_with("custom:") {selected} else {"general"};
            let items=command["items"].as_array().ok_or(Error::InvalidRequest)?;
            let preset=selected.strip_prefix("custom:").and_then(|id|items.iter().find(|v|v["id"]==id));
            let prompt=command["prompt"].as_str().unwrap_or("");
            let instruction=if selected=="general" {(!prompt.trim().is_empty() && prompt.trim().encode_utf16().count()>=3).then_some(prompt)}
                else {preset.and_then(|v|v["instruction"].as_str())};
            json!({"selected":selected,"instruction":instruction,"migration":selected.starts_with("custom:"),
                "migrationPrompt":preset.and_then(|v|v["instruction"].as_str()),
                "canSelect":items.iter().any(|v|v["id"]==command["id"])})
        },
        "presets" => {
            let raw=command["items"].as_array().ok_or(Error::InvalidRequest)?;
            let mut items:Vec<Value>=raw.iter().take(4).filter_map(|item|{
                let id=text(item,"id");let name=take(&text(item,"name").trim().split_ascii_whitespace().collect::<Vec<_>>().join(" "),24);
                let instruction=text(item,"instruction").trim();
                (!id.trim().is_empty() && !name.is_empty() && instruction.encode_utf16().count()>=3)
                    .then(||json!({"id":id,"name":name,"instruction":instruction}))}).collect();
            let mut preset=Value::Null;
            if command["delete"]==true {items.retain(|i|i["id"]!=command["id"])}
            else if command.get("name").is_some() {
                let name=take(&text(&command,"name").trim().split_ascii_whitespace().collect::<Vec<_>>().join(" "),24);
                let instruction=text(&command,"instruction").trim();let index=items.iter().position(|i|i["id"]==command["id"]);
                if !name.is_empty() && instruction.encode_utf16().count()>=3 && (index.is_some() || items.len()<4) {
                    preset=json!({"id":index.map(|i|items[i]["id"].clone()).unwrap_or(command["newId"].clone()),"name":name,"instruction":instruction});
                    if let Some(i)=index {items[i]=preset.clone()} else {items.push(preset.clone())}
                }
            }
            json!({"items":items,"preset":preset})
        },
        _=>return Err(Error::InvalidRequest),
    })
}
