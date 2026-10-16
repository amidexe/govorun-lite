use serde_json::{json, Value};

/// Display facts from an admitted server catalog. Native clients format prices
/// and dates, but never invent trial eligibility or a fixed plan multiplier.
pub(crate) fn project(catalog: &mut Value) {
    let free = crate::access_offer::FreeAccessOffer::from_profile(catalog, false);
    if catalog["starterGrant"].is_object() {
        // A signed-out catalog may advertise an offer, never authorize activation.
        catalog["starterGrant"]["clientOfferVisible"] = json!(
            !free.program_visible && catalog["starterGrant"]["state"] == "eligible");
    }
    let trial = &catalog["trialOffer"];
    catalog["clientTrialDays"] = if trial["state"] == "available" && trial["eligible"] == true {
        trial["durationDays"].clone()
    } else { json!(0) };
    let offers = catalog["offers"].as_array().expect("admitted catalog").clone();
    for offer in catalog["offers"].as_array_mut().expect("admitted catalog") {
        let multiplier = multiplier(offer, &offers);
        offer["clientMultiplierCopy"] = json!(multiplier.map(|n| format!("×{n}")));
        offer["clientLimitCopy"] = json!(match multiplier {
            Some(n) => format!("Лимиты в {n} {} больше", if (2..=4).contains(&(n % 10))
                && !(12..=14).contains(&(n % 100)) { "раза" } else { "раз" }),
            None if offer["planCode"] == "base" => "Для повседневной диктовки".into(),
            None => String::new(),
        });
    }
}

fn multiplier(offer: &Value, offers: &[Value]) -> Option<u64> {
    if offer["planCode"] != "pro" { return None; }
    let base: Vec<_> = offers.iter().filter(|base| base["planCode"] == "base"
        && base["periodDays"] == offer["periodDays"]).collect();
    let first = base.first()?;
    // Several independent base offers need not share the same limits.
    if base.iter().any(|base| base["dailyLimit"] != first["dailyLimit"]
        || base["periodLimit"] != first["periodLimit"]) { return None; }
    let daily = offer["dailyLimit"].as_u64()?;
    let period = offer["periodLimit"].as_u64()?;
    let base_daily = first["dailyLimit"].as_u64()?;
    let base_period = first["periodLimit"].as_u64()?;
    if base_daily == 0 || base_period == 0 || daily % base_daily != 0 || period % base_period != 0 {
        return None;
    }
    let multiplier = daily / base_daily;
    (multiplier > 1 && period / base_period == multiplier).then_some(multiplier)
}
