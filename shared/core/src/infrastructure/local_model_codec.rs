//! Current local scalar codecs. Platforms supply syntax/clock facts; Core owns
//! defaults and Retry-After policy. No storage, authority or network effects.
use crate::{Error,Result};
use serde_json::{json,Value};
pub(crate) fn evaluate(c:Value)->Result<Value> {
    let value=c["value"].as_str().unwrap_or("");
    let kind=c["kind"].as_str().ok_or(Error::InvalidRequest)?;
    if kind=="exchangeTiming" {return Ok(json!({"value":value.ends_with("/v2/stylize") || value.ends_with("/v1/stylize-status")}));}
    if kind=="retry" {
        let warmup=c["policy"]=="account";
        let maximum=if warmup {30_000} else {1_000_000_000_000_000i64};
        let v=value.trim();
        let delay=if v.is_empty() || (!warmup && v.chars().count()>128) {None}
            else if let Ok(seconds)=v.parse::<i64>() {if seconds<0 {None} else {Some(seconds.min(maximum/1000)*1000)}}
            else {c["dateMillis"].as_i64().zip(c["nowMillis"].as_i64()).and_then(|(date,now)|date.checked_sub(now)).map(|ms|ms.max(0))};
        return Ok(json!({"millis":delay.map(|ms|ms.clamp(1000,maximum))}));
    }
    let (known,fallback):(&[&str],&str)=match kind {
        "style"=>(&["formal","light","concise","custom"],"formal"),
        "punctuation"=>(&["less","normal"],"normal"),
        "emoji"=>(&["off","restrained","lively","bright"],if c["legacyEnabled"]==true {"bright"} else {"off"}),
        "resolvedEmoji"=>(&["off","restrained","lively","bright"],"off"),
        "theme"=>(&["light","dark","system"],"system"),
        "marker"=>(&["off","microphone","text"],"off"),
        _=>return Err(Error::InvalidRequest),
    };
    let canonical=if known.contains(&value) {value} else {fallback};
    Ok(json!({"value":if kind=="resolvedEmoji" && canonical=="off" && c["legacyEnabled"]==true {"bright"} else {canonical}}))
}
