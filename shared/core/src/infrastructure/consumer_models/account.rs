//! Typed consumer views of admitted Core views; no transport or authority.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerStyleAccess {
    #[serde(default = "style_access_type")]
    pub r#type: String,
    #[serde(default = "style_access_state")]
    pub state: String,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub ends_at: String,
    #[serde(default)]
    pub daily_resets_at: String,
    #[serde(default)]
    pub period_resets_at: String,
    #[serde(default)]
    pub daily_limit: i32,
    #[serde(default)]
    pub total_limit: i32,
    #[serde(default)]
    pub issued_today: i32,
    #[serde(default)]
    pub issued_total: i32,
    #[serde(default)]
    pub remaining_today: i32,
    #[serde(default)]
    pub remaining_total: i32,
    #[serde(default)]
    pub quota_unit: Option<String>,
    #[serde(default)]
    pub characters_per_credit: i32,
    #[serde(default)]
    pub maximum_credits_per_request: i32,
    #[serde(default)]
    pub stylization_protocol: Option<String>,
    #[serde(default)]
    pub plan_code: String,
    #[serde(default)]
    pub balance_kind: String,
    #[serde(default)]
    pub usage_balance: Option<ConsumerStyleUsageBalance>,
    #[serde(default)]
    pub reserve_return_protocol: Option<String>,
    #[serde(default)]
    pub next_period_at: String,
    #[serde(default)]
    pub period_reset_condition: String,
    #[serde(default)]
    pub access_ends_at: String,
    #[serde(default)]
    pub issuance_counters_observed: bool,
}
impl Default for ConsumerStyleAccess {
    fn default() -> Self { Self { r#type: "none".into(), state: "none".into(), starts_at: "".into(), ends_at: "".into(), daily_resets_at: "".into(), period_resets_at: "".into(), daily_limit: 0, total_limit: 0, issued_today: 0, issued_total: 0, remaining_today: 0, remaining_total: 0, quota_unit: None, characters_per_credit: 0, maximum_credits_per_request: 0, stylization_protocol: None, plan_code: "".into(), balance_kind: "".into(), usage_balance: None, reserve_return_protocol: None, next_period_at: "".into(), period_reset_condition: "".into(), access_ends_at: "".into(), issuance_counters_observed: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerOnlineConsent {
    #[serde(default)]
    pub accepted_version: i32,
    #[serde(default)]
    pub accepted_at: String,
    #[serde(default)]
    pub active: bool,
}
impl Default for ConsumerOnlineConsent {
    fn default() -> Self { Self { accepted_version: 0, accepted_at: "".into(), active: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerAccountDevice {
    pub id: String,
    pub platform: String,
    pub display_name: String,
    pub activated_at: String,
    pub last_seen_at: String,
    #[serde(default)]
    pub current: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerProfile {
    pub id: String,
    pub display_name: String,
    pub current_device_id: String,
    pub device_limit: i32,
    pub online_terms_version: i32,
    pub style_access: ConsumerStyleAccess,
    pub online_consent: ConsumerOnlineConsent,
    #[serde(default = "profile_online_service_state")]
    pub online_service_state: String,
    #[serde(default = "profile_starter_grant")]
    pub starter_grant: ConsumerStarterGrant,
    #[serde(default = "profile_promo_program")]
    pub promo_program: ConsumerPromoProgram,
    #[serde(default)]
    pub client_free_access_available: bool,
    #[serde(default)]
    pub style_usage_accounting_available: bool,
    #[serde(default)]
    pub has_synced_data: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerStyleUsageBalance {
    #[serde(default = "usage_balance_version")]
    pub version: i32,
    pub scope_id: String,
    pub revision: i64,
    pub day: String,
    pub server_time: String,
    pub consumed_today: i32,
    pub consumed_total: i32,
    pub remaining_today: i32,
    pub remaining_total: i32,
    #[serde(default)]
    pub device_days: std::collections::BTreeMap<String, i32>,
    #[serde(default)]
    pub settlement_scope_ids: Vec<String>,
    #[serde(default)]
    pub daily_settlement_scope_ids: Vec<String>,
    #[serde(default)]
    pub device_requests: std::collections::BTreeMap<String, ConsumerStyleRequestCount>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerStyleRequestCount {
    pub count: i32,
    pub credits: i32,
}

fn style_access_type() -> String { "none".into() }
fn usage_balance_version() -> i32 { 1 }
fn style_access_state() -> String { "none".into() }
fn profile_online_service_state() -> String { "unknown".into() }
fn profile_starter_grant() -> ConsumerStarterGrant { ConsumerStarterGrant::default() }
fn profile_promo_program() -> ConsumerPromoProgram { ConsumerPromoProgram::default() }
