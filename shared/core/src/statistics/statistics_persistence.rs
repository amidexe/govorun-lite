//! Keep stable's closed day DTO readable. Optional quality counters live in
//! the existing opaque projection; old writers may discard unsupported metrics.
use serde_json::{json, Map, Value};
const KEY: &str = "styleCounters";

fn take_days(value: &mut Value) -> Option<Value> {
    let mut quality = Map::new();
    for (date, day) in value.as_object_mut()? {
        let Some(fields) = day.as_object_mut() else { continue };
        let attempts = fields.remove("styleAttempts").and_then(|v| v.as_i64()).unwrap_or(0);
        let failures = fields.remove("styleFailures").and_then(|v| v.as_i64()).unwrap_or(0);
        let slow = fields.remove("styleSlow").and_then(|v| v.as_i64()).unwrap_or(0);
        if attempts > 0 { quality.insert(date.clone(), json!([attempts, failures, slow])); }
    }
    (!quality.is_empty()).then_some(Value::Object(quality))
}
fn restore_days(value: &mut Value, quality: &Value) {
    let Some(days) = value.as_object_mut() else { return };
    for (date, day) in days {
        let Some(counts) = quality[date].as_array().filter(|v| v.len() == 3) else { continue };
        let (Some(attempts), Some(failures), Some(slow)) = (counts[0].as_i64(), counts[1].as_i64(), counts[2].as_i64()) else { continue };
        if !(0..=crate::statistics_sync::MAX_TOTAL).contains(&attempts) || !(0..=attempts).contains(&failures) || !(0..=attempts-failures).contains(&slow) { continue; }
        if let Some(fields) = day.as_object_mut() {
            fields.insert("styleAttempts".into(), json!(attempts));
            fields.insert("styleFailures".into(), json!(failures));
            fields.insert("styleSlow".into(), json!(slow));
        }
    }
}
pub(crate) fn restore(mut state: Value, projection: &Value) -> Value {
    let quality = &projection[KEY];
    if state["epoch"].as_i64().is_none() || state["epoch"] != quality["epoch"] { return state; }
    restore_days(&mut state["days"], &quality["days"]);
    if let Some(accounts) = state["accounts"].as_object_mut() {
        for (id, account) in accounts {
            for field in ["acknowledgedDays", "carriedDays"] {
                restore_days(&mut account[field], &quality["accounts"][id][field]);
            }
        }
    }
    state
}
pub(super) fn decode(value: Value) -> Value {
    if value.get("state").is_some() && value.get("projection").is_some()
        && value.get("schemaVersion").is_none_or(|version| version == 1) {
        restore(value["state"].clone(), &value["projection"])
    } else { value }
}
pub(super) fn encode(state: &mut Value, projection: &mut Value) {
    let mut quality = json!({"epoch":state["epoch"]});
    if let Some(days) = take_days(&mut state["days"]) { quality["days"] = days; }
    if let Some(accounts) = state["accounts"].as_object_mut() {
        for (id, account) in accounts {
            for field in ["acknowledgedDays", "carriedDays"] {
                if let Some(days) = take_days(&mut account[field]) { quality["accounts"][id][field] = days; }
            }
        }
    }
    if quality.as_object().is_some_and(|v| v.len() > 1) { projection[KEY] = quality; }
}
