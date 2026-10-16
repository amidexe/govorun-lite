use chrono::DateTime;
use serde_json::{json, Value};

/// Presentation of server-owned eligibility. It never creates or extends access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreeAccessOffer {
    pub program_visible: bool,
    pub program_available: bool,
    pub starter_available: bool,
}

impl FreeAccessOffer {
    pub fn from_profile(profile: &Value, fresh: bool) -> Self {
        let program = &profile["promoProgram"];
        // Unknown or incomplete offers must not silently select a different grant.
        let program_visible = !program.is_null()
            && (program["state"] != "disabled" || program["participated"] == true);
        let program_available = fresh && program_eligible(program);
        let starter_available = fresh
            && !program_visible
            && profile["starterGrant"]["state"] == "eligible"
            && profile["starterGrant"]["activationAvailable"] == true;
        Self {
            program_visible,
            program_available,
            starter_available,
        }
    }

    pub fn available(self) -> bool {
        self.program_available || self.starter_available
    }
}

fn program_eligible(program: &Value) -> bool {
    program["state"] == "active"
        && program["activationAvailable"] == true
        && program["enrollmentOpen"] == true
        && program["participated"] == false
        && ["campaignId", "programVersion"].iter().all(|key| {
            program[key]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
        })
        && ["requiresPaymentMethod", "automaticallyRenews"]
            .iter()
            .all(|key| program.get(key).is_none_or(|v| v.is_null() || v == false))
        && integer(&program["accessDays"], 1, 90)
        && integer(&program["periodDays"], 1, 90)
        && integer(&program["dailyLimit"], 1, i32::MAX as u64)
        && integer(
            &program["periodLimit"],
            program["dailyLimit"].as_u64().unwrap_or(u64::MAX),
            i32::MAX as u64,
        )
        && ["accessEndsAt", "endsAt"]
            .iter()
            .all(|key| program.get(key).is_none_or(|v| v.is_null() || timestamp(v)))
        && (program["accessMode"] == "from_activation"
            || program["accessMode"] == "common_end" && timestamp(&program["accessEndsAt"]))
}

fn integer(value: &Value, min: u64, max: u64) -> bool {
    value.as_u64().is_some_and(|v| (min..=max).contains(&v))
}

fn timestamp(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|v| DateTime::parse_from_rfc3339(v).is_ok())
}

/// Private adapter DTO. Recompute these fields; never trust client-prefixed wire input.
/// Numbers, dates and messages remain exactly the server's values.
pub fn project_free_access(mut wire: Value, fresh: bool) -> Value {
    let offer = FreeAccessOffer::from_profile(&wire, fresh);
    wire["clientFreeAccessAvailable"] = json!(offer.available());
    if wire["promoProgram"].is_object() {
        wire["promoProgram"]["clientVisible"] = json!(offer.program_visible);
        wire["promoProgram"]["clientCanActivate"] = json!(offer.program_available);
    }
    if wire["starterGrant"].is_object() {
        wire["starterGrant"]["clientCanActivate"] = json!(offer.starter_available);
    }
    wire
}

pub(crate) fn project_catalog(wire: Value) -> Value {
    let mut result = project_free_access(wire, false);
    crate::catalog_presentation::project(&mut result);
    result
        .as_object_mut()
        .expect("validated catalog")
        .remove("clientFreeAccessAvailable");
    result
}
