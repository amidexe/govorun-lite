use super::{accepts,number,text};
use crate::Result;
use serde_json::{json,Value};
const FIELDS:[&str;10]=["schema","id","package","category","title","body","minVersionCode","maxVersionCode","publishedAt","expiresAt"];
pub(super) fn parse(d:&Value,app:&str,version:i64,now:i64)->Option<Value> {
    let fields=d.as_object()?;
    if fields.len()!=FIELDS.len() || !FIELDS.iter().all(|k|fields.contains_key(*k)) || text(d,"schema")!="1" || text(d,"package")!=app {return None;}
    let id=text(d,"id");let category=text(d,"category");let title=text(d,"title");let body=text(d,"body");
    let start=text(d,"publishedAt").parse::<i64>().ok()?;let end=text(d,"expiresAt").parse::<i64>().ok()?;
    let min=text(d,"minVersionCode").parse::<i64>().ok()?;let max=text(d,"maxVersionCode").parse::<i64>().ok()?;
    if id.is_empty() || id.len()>96 || !id.as_bytes()[0].is_ascii_alphanumeric() ||
        !id.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)) ||
        !matches!(category,"updates"|"offers") || title.trim().is_empty() || title.chars().count()>100 ||
        body.trim().is_empty() || body.chars().count()>500 || min<1 || max<min || !(min..=max).contains(&version) ||
        start<=0 || start>now || end<=now || end<=start || end.checked_sub(start)?>28*86400 {return None;}
    Some(json!({"id":id,"category":category,"title":title,"body":body,"publishedAt":start,"expiresAt":end}))
}
pub(super) fn receive(mut s:Value,n:Value,permission:bool,now:i64)->Result<Value> {
    let category=text(&n,"category");let id=text(&n,"id");
    let mut seen=s["seen"].as_object().cloned().unwrap_or_default();seen.retain(|_,v|v.as_i64().unwrap_or(0)>now);
    if !permission || !accepts(&s,category) || number(&n,"expiresAt")<=now || seen.contains_key(id) || seen.len()>=256 ||
        number(&s["notices"][category],"publishedAt")>number(&n,"publishedAt") {return Ok(Value::Null);}
    seen.insert(id.into(),n["expiresAt"].clone());s["seen"]=json!(seen);
    if !s["notices"].is_object() {s["notices"]=json!({});} s["notices"][category]=n.clone(); Ok(s)
}
