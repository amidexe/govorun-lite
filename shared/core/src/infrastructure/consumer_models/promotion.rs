//! Typed consumer views of admitted Core views; no transport or authority.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerStarterGrant {
    #[serde(default = "starter_grant_state")]
    pub state: String,
    #[serde(default)]
    pub credit_limit: i32,
    #[serde(default)]
    pub remaining_to_issue: i32,
    #[serde(default)]
    pub activation_available: bool,
    #[serde(default)]
    pub expires: bool,
    #[serde(default)]
    pub balance_kind: String,
    #[serde(default)]
    pub trial_offer: Option<ConsumerBillingTrialOffer>,
    #[serde(default)]
    #[serde(alias = "clientCanActivate")]
    pub eligible: bool,
}
impl Default for ConsumerStarterGrant {
    fn default() -> Self { Self { state: "disabled".into(), credit_limit: 0, remaining_to_issue: 0, activation_available: false, expires: false, balance_kind: "".into(), trial_offer: None, eligible: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoProgram {
    #[serde(default)]
    pub campaign_id: String,
    #[serde(default)]
    pub program_version: String,
    #[serde(default = "promo_program_state")]
    pub state: String,
    #[serde(default)]
    pub enrollment_open: bool,
    #[serde(default)]
    pub activation_available: bool,
    #[serde(default)]
    pub participated: bool,
    #[serde(default = "promo_program_access_mode")]
    pub access_mode: String,
    #[serde(default)]
    pub access_days: i32,
    #[serde(default)]
    pub access_ends_at: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub daily_limit: i32,
    #[serde(default)]
    pub period_limit: i32,
    #[serde(default)]
    pub period_days: i32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    #[serde(alias = "clientVisible")]
    pub visible: bool,
    #[serde(default)]
    #[serde(alias = "clientCanActivate")]
    pub can_activate: bool,
}
impl Default for ConsumerPromoProgram {
    fn default() -> Self { Self { campaign_id: "".into(), program_version: "".into(), state: "disabled".into(), enrollment_open: false, activation_available: false, participated: false, access_mode: "common_end".into(), access_days: 0, access_ends_at: None, ends_at: None, daily_limit: 0, period_limit: 0, period_days: 0, message: "".into(), visible: false, can_activate: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingStorefront {
    #[serde(default = "billing_storefront_state")]
    pub state: String,
    #[serde(default)]
    pub message: String,
}
impl Default for ConsumerBillingStorefront {
    fn default() -> Self { Self { state: "open".into(), message: "".into() } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoAccessDays {
    pub days: i32,
    pub plan_code: String,
    pub plan_name: String,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoBillingPeriod {
    pub number: i32,
    pub standard_price_kopeks: i32,
    pub payable_price_kopeks: i32,
    pub discount_amount_kopeks: i32,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoBonus {
    pub credits: i32,
    pub valid_days: i32,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoOffer {
    pub plan_code: String,
    pub plan_name: String,
    pub currency: String,
    #[serde(default)]
    pub billing_periods: Vec<ConsumerPromoBillingPeriod>,
    #[serde(default)]
    pub following_price_kopeks: i32,
    #[serde(default)]
    pub following_period_days: i32,
    pub bonus: Option<ConsumerPromoBonus>,
    pub confirmation_title: String,
    pub confirmation_body: String,
    #[serde(default)]
    pub creates_subscription: bool,
    #[serde(default)]
    pub checkout_required: bool,
    #[serde(default)]
    pub grants_access_on_confirmation: bool,
    #[serde(default = "promo_offer_catalog_version")]
    pub catalog_version: String,
    #[serde(default)]
    pub offer_id: String,
    #[serde(default)]
    pub access_days: Option<ConsumerPromoAccessDays>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoOfferPreview {
    pub preview_id: String,
    pub preview_expires_at: String,
    pub offer: ConsumerPromoOffer,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerPromoRedemption {
    pub redemption_id: String,
    pub redeemed_at: String,
    pub offer: ConsumerPromoOffer,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerStarterGrantOffer {
    #[serde(default)]
    pub client_offer_visible: bool,
    #[serde(default = "starter_grant_offer_state")]
    pub state: String,
    #[serde(default)]
    pub credit_limit: i32,
    #[serde(default = "starter_grant_offer_requires_authentication")]
    pub requires_authentication: bool,
    #[serde(default)]
    pub requires_code: bool,
    #[serde(default)]
    pub requires_payment_method: bool,
    #[serde(default)]
    pub expires: bool,
}
impl Default for ConsumerStarterGrantOffer {
    fn default() -> Self { Self { client_offer_visible: false, state: "disabled".into(), credit_limit: 0, requires_authentication: true, requires_code: false, requires_payment_method: false, expires: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingTrialOffer {
    #[serde(default = "billing_trial_offer_state")]
    pub state: String,
    #[serde(default)]
    pub eligible: bool,
    #[serde(default)]
    pub duration_days: i32,
    #[serde(default = "billing_trial_offer_requires_verified_payment_method")]
    pub requires_verified_payment_method: bool,
    #[serde(default)]
    pub checkout_open_grants_access: bool,
    #[serde(default)]
    pub requires_local_reserve_empty: bool,
}
impl Default for ConsumerBillingTrialOffer {
    fn default() -> Self { Self { state: "disabled".into(), eligible: false, duration_days: 0, requires_verified_payment_method: true, checkout_open_grants_access: false, requires_local_reserve_empty: false } }
}

fn starter_grant_state() -> String { "disabled".into() }
fn promo_program_state() -> String { "disabled".into() }
fn promo_program_access_mode() -> String { "common_end".into() }
fn billing_storefront_state() -> String { "open".into() }
fn promo_offer_catalog_version() -> String { "test".into() }
fn starter_grant_offer_state() -> String { "disabled".into() }
fn starter_grant_offer_requires_authentication() -> bool { true }
fn billing_trial_offer_state() -> String { "disabled".into() }
fn billing_trial_offer_requires_verified_payment_method() -> bool { true }
