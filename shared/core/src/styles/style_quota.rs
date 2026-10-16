use serde::Serialize;
use serde_json::Value;

/// Shared meaning of the quota card. Clients format timestamps in the user's
/// locale and arrange the fields, without choosing balances or renewal rules.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotaPresentation {
    pub kind: &'static str,
    pub plan_title: &'static str,
    pub heading: &'static str,
    pub today_percent: Option<u8>,
    pub period_percent: Option<u8>,
    pub refreshing: bool,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub today_reset: Option<String>,
    pub period_status: DateCopy,
    pub promo_active: bool,
    pub paid_plan_code: Option<&'static str>,
}

#[derive(Debug, Default, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct DateCopy {
    pub prefix: &'static str,
    pub timestamp: Option<String>,
    pub include_time: bool,
}

fn timestamp(access: &Value, key: &str) -> Option<String> {
    let value = access[key].as_str()?;
    chrono::DateTime::parse_from_rfc3339(value).ok()?;
    Some(value.to_owned())
}

fn percentage(remaining: u64, limit: u64) -> u8 {
    if remaining == 0 || limit == 0 { return 0; }
    ((u128::from(remaining.min(limit)) * 100).div_ceil(u128::from(limit))) as u8
}

pub(crate) fn project(access: &Value, signed_in: bool, eligible: bool, refreshing: bool,
    remaining_today: u64, remaining_total: u64, daily_limit: u64, total_limit: u64) -> QuotaPresentation {
    // Cached server metadata may identify the plan, but never supplies fresh
    // percentages or processing authority before a confirmed observation.
    let active = signed_in && access["state"] == "active";
    let refreshing = refreshing || !eligible;
    let access_type = access["type"].as_str().unwrap_or("");
    let plan = access["planCode"].as_str().unwrap_or("");
    let kind = if !active { "none" } else {
        match access["balanceKind"].as_str() {
            Some("single_grant") => "singleGrant",
            Some("single_bonus") => "singleBonus",
            Some("daily_and_period") => "period",
            _ => "none",
        }
    };
    let plan_title = if !active { "Без подписки" } else { match (plan, access_type) {
        ("pro", _) => "Про",
        ("base", _) => "Базовая",
        (_, "starter") => "Бесплатный лимит",
        (_, "promo" | "bonus") => "Бонусный лимит",
        (_, "promo_program") => "Промо",
        (_, "trial") => "Пробный период",
        (_, "closed_beta") => "Тестовый доступ",
        _ => "Доступ",
    }};
    let heading = match kind {
        "singleGrant" => "Бесплатный лимит", "singleBonus" => "Бонусный лимит",
        _ => "Остаток",
    };
    let period = kind == "period";
    let known = kind != "none" && !refreshing;
    let period_start = period.then(|| timestamp(access, "startsAt")).flatten();
    let period_end = period.then(|| timestamp(access, "endsAt")).flatten();
    let has_range = period_start.is_some() && period_end.is_some();
    QuotaPresentation {
        kind, plan_title, heading,
        today_percent: (known && period).then(|| percentage(remaining_today, daily_limit)),
        period_percent: known.then(|| percentage(remaining_total, total_limit)),
        refreshing: kind != "none" && refreshing,
        period_start: has_range.then_some(period_start).flatten(),
        period_end: has_range.then_some(period_end).flatten(),
        today_reset: (period && known).then(|| timestamp(access, "dailyResetsAt")).flatten(),
        period_status: period_status(access, kind),
        promo_active: active && matches!(access_type, "promo" | "trial" | "closed_beta"),
        paid_plan_code: if active && matches!(access_type, "paid" | "plan_grant") {
            match plan { "base" => Some("base"), "pro" => Some("pro"), _ => None }
        } else { None },
    }
}

fn period_status(access: &Value, kind: &str) -> DateCopy {
    let dated = |prefix, key, include_time| {
        let timestamp = timestamp(access, key);
        DateCopy { prefix: if timestamp.is_some() { prefix } else { "" }, timestamp, include_time }
    };
    if kind == "singleBonus" { return dated("Действует до ", "endsAt", false); }
    if kind != "period" { return DateCopy::default(); }
    let condition = access["periodResetCondition"].as_str().unwrap_or("");
    if condition == "payment_pending" {
        return DateCopy { prefix: "Обновится после подтверждения оплаты", ..Default::default() };
    }
    if timestamp(access, "nextPeriodAt").is_some() {
        match condition {
            "automatic" => return dated("Обновится ", "nextPeriodAt", true),
            "subscription_renewal" => return dated("При продлении ", "nextPeriodAt", true),
            _ => (),
        }
    }
    if timestamp(access, "accessEndsAt").is_some() { return dated("Доступ до ", "accessEndsAt", false); }
    let prefix = match access["type"].as_str() {
        Some("promo_program") => "Период до ", Some("trial") => "Пробный период до ",
        Some("promo" | "bonus") => "Бонус до ", _ => "Доступ до ",
    };
    dated(prefix, "endsAt", false)
}
