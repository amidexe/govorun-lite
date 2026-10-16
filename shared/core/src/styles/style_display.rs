use crate::{Error, Result};
use serde::Serialize;

/// State decisions for desktop consumers. Translated copy and typography remain
/// rendering concerns; neither message text nor local preset counts are authority.
#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub(crate) struct Display {
    pub availability_reason: &'static str,
    pub display_state: &'static str,
    pub limit_reason: &'static str,
    pub max_presets: usize,
    pub can_create_preset: bool,
    pub resume_choice: bool,
}

pub(crate) fn project(gate: &str, availability: &'static str, enabled: bool,
    paused: bool, readiness: &str, preset_count: u32, self_hosted: bool) -> Result<Display> {
    if !matches!(readiness, "ready" | "onDemand" | "preparing" | "needsPreparation" | "unavailable") {
        return Err(Error::InvalidRequest);
    }
    let preparing = enabled && gate == "ready" && matches!(readiness, "preparing" | "needsPreparation");
    let active = enabled && !paused && (gate == "selfHosted" ||
        gate == "ready" && matches!(readiness, "ready" | "onDemand"));
    let state = if !enabled { "disabled" } else if preparing { "preparing" }
        else if active { "active" } else { "saved" };
    let limit = if gate != "unavailable" { "none" } else { match availability {
        "periodLimit" => "period", "dailyLimit" => "daily", _ => "none",
    }};
    Ok(Display { availability_reason: availability, display_state: state,
        limit_reason: limit,
        max_presets: crate::content_sync::MAX_CUSTOM_STYLES,
        can_create_preset: !self_hosted && (preset_count as usize) < crate::content_sync::MAX_CUSTOM_STYLES,
        resume_choice: matches!(gate, "ready" | "access" | "unavailable") })
}
