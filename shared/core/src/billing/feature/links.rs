//! Only host-selected destinations can reach the OS opener.
use super::{dto::BillingSnapshot, format::Dates};
use serde_json::Value;

pub fn https(value: &str) -> Result<&str, &'static str> {
    let url = url::Url::parse(value).map_err(|_| "invalid_parameters")?;
    if value.len() > 4096
        || value.bytes().any(|b| b.is_ascii_control())
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("invalid_parameters");
    }
    Ok(value)
}

pub fn legal(projection: &Value, params: &Value) -> Result<String, &'static str> {
    let key = params["document"].as_str().ok_or("invalid_parameters")?;
    if key == "onlineTerms" {
        return Ok("https://govorun.club/legal".into());
    }
    let window = &projection["subscriptionWindow"];
    if window["previewId"].as_str().is_none()
        || params["previewId"] != window["previewId"]
        || !matches!(
            key,
            "subscriptionTerms"
                | "userAgreement"
                | "refundPolicy"
                | "privacyNotice"
                | "receiptDataConsent"
        )
    {
        return Err("invalid_parameters");
    }
    Ok(https(
        window["preview"]["legalDocuments"][key]["url"]
            .as_str()
            .ok_or("invalid_parameters")?,
    )?
    .into())
}

pub fn support(projection: &Value, snapshot: &BillingSnapshot, dates: Dates) -> String {
    let subscription = &projection["billingStatus"]["status"]["subscription"];
    let mut lines = if subscription["state"] == "none" || !subscription.is_object() {
        vec!["Подписка: не подключена".into()]
    } else {
        vec![
            format!(
                "Тариф: {}",
                match subscription["planCode"].as_str() {
                    Some("base") => "Базовая",
                    Some("pro") => "Про",
                    _ => "уточняется",
                }
            ),
            format!("Статус: {}", snapshot.status.title),
        ]
    };
    if subscription.is_object() && subscription["state"] != "none" {
        let overview = &projection["subscriptionOverview"];
        if let Some(end) = dates.date(overview["periodEnd"].as_str(), true, false, true) {
            lines.push(format!(
                "{} {end}",
                overview["periodLabel"].as_str().unwrap_or("")
            ));
        }
        lines.push(snapshot.status.payment.clone());
        if let Some(notice) = &snapshot.status.renewal_notice {
            lines.push(notice.clone());
        }
    }
    let id = projection["profile"]["id"]
        .as_str()
        .filter(|id| valid_uuid(id))
        .unwrap_or("не определён");
    let body = format!(
        "{}\nКод аккаунта: {id}\n\nОпишите вопрос об оплате, отмене или возврате:",
        lines.join("\n")
    );
    let mut url = url::Url::parse("mailto:support@govorun.club").expect("fixed mailto");
    url.query_pairs_mut()
        .append_pair("subject", "Говорун — подписка")
        .append_pair("body", &body);
    url.into()
}
fn valid_uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

pub fn bank_return(value: &str, scheme: &str) -> bool {
    if value.len() > 4096 || value.bytes().any(|b| b.is_ascii_control()) {
        return false;
    }
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == scheme
            && url.host_str() == Some("billing")
            && url.path() == "/tbank-return"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.fragment().is_none()
    })
}
