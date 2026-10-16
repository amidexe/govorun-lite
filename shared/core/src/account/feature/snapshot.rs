//! Present only admitted session/device facts; credentials never reach this DTO.
use super::dto::*;
use chrono::{DateTime, Datelike, Local};
use serde_json::Value;

fn moment(value: &str, relative: bool, now: &DateTime<Local>) -> Result<String, &'static str> {
    let date = DateTime::parse_from_rfc3339(value)
        .map_err(|_| "account_invalid_response")?
        .with_timezone(&Local);
    let time = date.format("%H:%M");
    if relative && date.date_naive() == now.date_naive() {
        return Ok(format!("сегодня, {time}"));
    }
    if relative && Some(date.date_naive()) == now.date_naive().pred_opt() {
        return Ok(format!("вчера, {time}"));
    }
    let months = [
        "янв.",
        "февр.",
        "мар.",
        "апр.",
        "мая",
        "июн.",
        "июл.",
        "авг.",
        "сент.",
        "окт.",
        "нояб.",
        "дек.",
    ];
    let year = if date.year() == now.year() {
        String::new()
    } else {
        format!(" {}", date.year())
    };
    Ok(format!(
        "{} {}{year}, {time}",
        date.day(),
        months[date.month0() as usize]
    ))
}
fn devices(list: &Value, now: &DateTime<Local>) -> Result<Vec<AccountDevice>, &'static str> {
    let Some(values) = list["devices"].as_array() else {
        return Ok(vec![]);
    };
    values.iter().map(|value| {
        let read = |key| value[key].as_str().ok_or("account_invalid_response");
        let current = value["current"] == true;
        let platform = match read("platform")? {"android"=>"Android","macos"=>"macOS","windows"=>"Windows",_=>"Устройство"};
        let activity = moment(read("lastSeenAt")?,true,now)?;
        let activation = moment(read("activatedAt")?,false,now)?;
        Ok(AccountDevice {id:read("id")?.into(),name:read("displayName")?.into(),current,
            subtitle:if current {"Это устройство".into()} else {format!("{platform} · {activity}")},
            confirmation:format!("Последняя активность — {activity}. Добавлено — {activation}. Доступ к аккаунту и онлайн-функциям будет отозван. Локальные данные на устройстве останутся.")})
    }).collect()
}

pub fn snapshot(
    projection: &Value,
    facts: &Value,
    scope: String,
    busy: bool,
    waiting: bool,
    message_code: Option<String>,
) -> Result<AccountSnapshot, &'static str> {
    snapshot_at(projection, facts, scope, busy, waiting, message_code, &Local::now())
}

/// Project device dates using the caller-provided clock, for deterministic adapters and tests.
pub fn snapshot_at(
    projection: &Value, facts: &Value, scope: String, busy: bool, waiting: bool,
    message_code: Option<String>, now: &DateTime<Local>,
) -> Result<AccountSnapshot, &'static str> {
    let account = &projection["snapshot"];
    let signed_in = account["signedIn"] == true;
    let pending = &projection["pendingDeviceLimit"];
    let replacing = pending.is_object();
    let list = if replacing {
        pending
    } else {
        &projection["devices"]
    };
    let storage = facts["storageState"]
        .as_str()
        .ok_or("account_invalid_response")?;
    if !matches!(
        storage,
        "available" | "permissionRequired" | "temporarilyUnavailable" | "invalidData"
    ) {
        return Err("account_invalid_response");
    }
    let storage = if account["storageAvailable"] != true && storage == "available" {
        "temporarilyUnavailable"
    } else {
        storage
    };
    let sync = facts["syncMode"]
        .as_str()
        .ok_or("account_invalid_response")?;
    if !matches!(sync, "evaluating" | "pending" | "enabled" | "localOnly") {
        return Err("account_invalid_response");
    }
    let message = if replacing {
        Some("account_device_limit".into())
    } else {
        message_code
    };
    let status = if signed_in {
        "signedIn"
    } else if waiting || account["signingIn"] == true {
        "signingIn"
    } else if message.is_some() {
        "failed"
    } else {
        "signedOut"
    };
    let providers = facts["providers"]
        .as_array()
        .ok_or("account_invalid_response")?
        .iter()
        .map(|provider| {
            let id = provider["id"]
                .as_str()
                .filter(|id| matches!(*id, "yandex" | "vk"))
                .ok_or("account_invalid_response")?;
            Ok(AccountProvider {
                id: id.into(),
                enabled: provider["enabled"]
                    .as_bool()
                    .ok_or("account_invalid_response")?,
            })
        })
        .collect::<Result<Vec<_>, &str>>()?;
    if providers.is_empty()
        || providers.len() > 2
        || providers.len() == 2 && providers[0].id == providers[1].id
    {
        return Err("account_invalid_response");
    }
    let limit = list["deviceLimit"]
        .as_u64()
        .or_else(|| projection["tokens"]["deviceLimit"].as_u64())
        .unwrap_or(5);
    if !(1..=100).contains(&limit) {
        return Err("account_invalid_response");
    }
    Ok(AccountSnapshot {
        scope,
        status: status.into(),
        display_name: if signed_in {
            account["displayName"].as_str().map(str::to_owned)
        } else {
            None
        },
        message_code: message,
        busy,
        sync_mode: sync.into(),
        providers,
        provider_notice_code: facts["providerNoticeCode"].as_str().map(str::to_owned),
        devices: devices(list, now)?,
        device_limit: limit as u32,
        replacing_device: replacing,
        storage: AccountStorage {
            available: storage == "available",
            state: storage.into(),
        },
    })
}
