//! Decode preceding numeric fields in Rust. Native ports supply raw values only.
use serde_json::{json, Value};
use std::collections::BTreeSet;

type Checked = Result<Value, ()>;
fn zero() -> Value { json!({"words":0,"speechSeconds":0,"recognitions":0}) }
fn raw(value: &Value, missing: Value) -> Checked {
    if value.is_null() { return Ok(missing); }
    let text = value.as_str().ok_or(())?;
    serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(text)).map_err(|_| ())
}
fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, ()> { value.as_object().ok_or(()) }
fn field(value: &Value, key: &str, default: Value) -> Value { value.get(key).cloned().unwrap_or(default) }

pub(super) fn previous(facts: &Value, account: Option<&str>, today: &str) -> Value {
    let result = match facts["source"].as_str() {
        Some("macos") => macos(&facts["values"], account, today),
        Some("windows") => windows(&facts["values"], account, today),
        _ => Err(()),
    };
    result.unwrap_or_else(|_| json!({"unreadable":true}))
}
fn windows(values: &Value, account: Option<&str>, today: &str) -> Checked {
    object(values)?;
    let days = raw(&values["days"], json!([]))?;
    let rows: Vec<crate::statistics_sync::DeviceDay> = serde_json::from_value(days.clone()).map_err(|_| ())?;
    let mut dates = BTreeSet::new();
    for row in &rows {
        row.validate(&row.date).map_err(|_| ())?;
        if !dates.insert(&row.date) { return Err(()); }
    }
    let account = account.map(|id| windows_account(&values["syncState"], id)
        .unwrap_or_else(|_| json!({"unreadable":true})));
    Ok(json!({"today":today,"total":null,"days":days,"account":account}))
}
fn windows_account(source: &Value, id: &str) -> Checked {
    let records = raw(source, json!({}))?; object(&records)?;
    let record = &records[id];
    let generation = field(record, "statsGeneration", json!(0));
    let total = record["acknowledgedStatsTotal"].as_object().map(|value| Value::Object(value.clone())).unwrap_or_else(zero);
    Ok(json!({"id":id,"generation":generation,"cloud":{
        "generation":generation,"cursor":field(record,"statsCursor",json!(0)),
        "total":field(record,"cloudStatsTotal",Value::Null),"days":field(record,"cloudStatsDays",json!([]))},
        "acknowledgedTotal":total,"acknowledgedDays":[],"resetPending":false}))
}
fn macos(values: &Value, account: Option<&str>, today: &str) -> Checked {
    object(values)?;
    let days = macos_days(values).unwrap_or(Value::Null);
    let account = account.map(|id| macos_account(values, id).unwrap_or_else(|_| json!({"unreadable":true})));
    Ok(json!({"today":today,"total":{"words":field(&values["total"],"words",json!(0)),
        "speechSeconds":field(&values["total"],"speechSeconds",json!(0)),"recognitions":field(&values["total"],"recognitions",json!(0))},
        "days":days,"account":account}))
}
fn macos_days(values: &Value) -> Checked {
    let history = raw(&values["history"], json!({}))?;
    let performance = raw(&values["performance"], json!({}))?;
    let dates: BTreeSet<_> = object(&history)?.keys().chain(object(&performance)?.keys()).collect();
    let mut days = Vec::new();
    for date in dates {
        let row = history.get(date).cloned().unwrap_or_else(|| json!({})); object(&row)?;
        let quality = performance.get(date).filter(|value| value.is_object()).cloned().unwrap_or_else(|| json!({}));
        days.push(json!({"date":date,"revision":1,"words":field(&row,"words",json!(0)),
            "speechSeconds":field(&row,"seconds",json!(0)),"recognitions":field(&row,"sessions",json!(0)),
            "recognitionRtfBuckets":field(&quality,"rtfBuckets",json!([0,0,0,0,0])),"recognitionErrors":field(&quality,"errors",json!(0))}));
    }
    Ok(json!(days))
}
fn macos_account(values: &Value, id: &str) -> Checked {
    let record = &values["accounts"][id];
    let cloud = raw(&record["cloud"], json!({}))?; object(&cloud)?;
    let total = raw(&record["acknowledgedTotal"], json!({}))?; object(&total)?;
    let days = raw(&record["acknowledgedDays"], json!({}))?;
    let acknowledged: Vec<_> = object(&days)?.values().cloned().collect();
    let snapshot = if object(&cloud)?.is_empty() { Value::Null } else { json!({
        "generation":field(&cloud,"generation",json!(0)),"cursor":field(&cloud,"cursor",json!(0)),
        "total":field(&cloud,"total",Value::Null),"days":field(&cloud,"days",json!([]))}) };
    let reset = match record.get("resetPending") {
        None => false, Some(Value::Bool(value)) => *value,
        Some(value) if value == &json!(0) => false, Some(value) if value == &json!(1) => true,
        _ => return Err(()),
    };
    Ok(json!({"id":id,"generation":field(&cloud,"generation",json!(0)),"cloud":snapshot,
        "acknowledgedTotal":if object(&total)?.is_empty() {zero()} else {total},
        "acknowledgedDays":acknowledged,"resetPending":reset}))
}
