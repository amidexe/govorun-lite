//! Typed consumer views of admitted Core views; no transport or authority.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingPaymentMethod {
    #[serde(default = "billing_payment_method_state")]
    pub state: String,
    #[serde(default)]
    pub last_four: Option<String>,
    #[serde(default)]
    pub can_remove: bool,
    #[serde(default = "billing_payment_method_kind")]
    pub kind: String,
    #[serde(default)]
    pub available: Option<bool>,
}
impl Default for ConsumerBillingPaymentMethod {
    fn default() -> Self { Self { state: "unknown".into(), last_four: None, can_remove: false, kind: "card".into(), available: None } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingPlan {
    #[serde(alias = "planCode")]
    pub code: String,
    #[serde(alias = "planName")]
    pub name: String,
    pub price_kopeks: i32,
    pub daily_limit: i32,
    pub period_limit: i32,
    pub period_days: i32,
    #[serde(default)]
    pub recommended: bool,
    #[serde(default)]
    pub ru_store_product_id: String,
    #[serde(default)]
    pub offer_id: String,
    #[serde(default)]
    pub billing_period_label: String,
    #[serde(default)]
    pub client_multiplier_copy: Option<String>,
    #[serde(default)]
    pub client_limit_copy: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingCatalog {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub catalog_version: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub refresh_after_seconds: i32,
    #[serde(default = "billing_catalog_currency")]
    pub currency: String,
    #[serde(default)]
    pub ru_store_enabled: bool,
    #[serde(default)]
    pub t_bank_enabled: bool,
    #[serde(default)]
    pub t_bank_preview_enabled: bool,
    #[serde(default = "billing_catalog_trial_offer")]
    pub trial_offer: ConsumerBillingTrialOffer,
    #[serde(default = "billing_catalog_starter_grant")]
    pub starter_grant: ConsumerStarterGrantOffer,
    #[serde(default)]
    #[serde(alias = "offers")]
    pub plans: Vec<ConsumerBillingPlan>,
    #[serde(default = "billing_catalog_storefront")]
    pub storefront: ConsumerBillingStorefront,
    #[serde(default)]
    pub program_control_version: String,
    #[serde(default = "billing_catalog_promo_program")]
    pub promo_program: ConsumerPromoProgram,
    #[serde(default)]
    pub client_trial_days: i32,
}
impl Default for ConsumerBillingCatalog {
    fn default() -> Self { Self { schema_version: 0, catalog_version: "".into(), published_at: "".into(), refresh_after_seconds: 0, currency: "RUB".into(), ru_store_enabled: false, t_bank_enabled: false, t_bank_preview_enabled: false, trial_offer: ConsumerBillingTrialOffer::default(), starter_grant: ConsumerStarterGrantOffer::default(), plans: Vec::new(), storefront: ConsumerBillingStorefront::default(), program_control_version: "".into(), promo_program: ConsumerPromoProgram::default(), client_trial_days: 0 } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingStatus {
    #[serde(default = "billing_status_subscription")]
    pub subscription: ConsumerBillingSubscription,
    #[serde(default = "billing_status_starter_grant")]
    pub starter_grant: ConsumerStarterGrant,
}
impl Default for ConsumerBillingStatus {
    fn default() -> Self { Self { subscription: ConsumerBillingSubscription::default(), starter_grant: ConsumerStarterGrant::default() } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingSubscription {
    #[serde(default = "billing_subscription_state")]
    pub state: String,
    #[serde(default)]
    pub plan_code: String,
    #[serde(default)]
    pub offer_id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub current_period_ends_at: String,
    #[serde(default)]
    pub renews_automatically: bool,
    #[serde(default)]
    pub pending_plan_code: String,
    #[serde(default)]
    pub pending_offer_id: String,
    #[serde(default)]
    pub change_effective_at: String,
    #[serde(default)]
    pub billing_subject_id: String,
    #[serde(default)]
    pub service_access_state: String,
    #[serde(default)]
    pub suspension_reason: String,
    #[serde(default)]
    pub suspended_until: String,
    #[serde(default)]
    pub refund_request: Option<ConsumerBillingRefundRequest>,
    #[serde(default)]
    pub renewal_issue: Option<ConsumerBillingRenewalIssue>,
    #[serde(default)]
    pub next_charge: Option<ConsumerBillingNextCharge>,
    #[serde(default)]
    pub payment_method: Option<ConsumerBillingPaymentMethod>,
}
impl Default for ConsumerBillingSubscription {
    fn default() -> Self { Self { state: "none".into(), plan_code: "".into(), offer_id: "".into(), provider: "".into(), current_period_ends_at: "".into(), renews_automatically: false, pending_plan_code: "".into(), pending_offer_id: "".into(), change_effective_at: "".into(), billing_subject_id: "".into(), service_access_state: "".into(), suspension_reason: "".into(), suspended_until: "".into(), refund_request: None, renewal_issue: None, next_charge: None, payment_method: None } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingRenewalIssue {
    pub state: String,
    #[serde(default)]
    pub resolution_kind: String,
    #[serde(default)]
    pub charge_kind: String,
    #[serde(default)]
    pub payment_state: String,
    #[serde(default)]
    pub amount_kopeks: i32,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub payment_method_saved: bool,
    pub attempt_count: i32,
    pub maximum_attempts: i32,
    #[serde(default)]
    pub occurred_at: String,
    #[serde(default)]
    pub next_attempt_at: String,
    #[serde(default)]
    pub access_ends_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingNextCharge {
    pub amount_kopeks: i32,
    pub currency: String,
    pub charge_at: String,
    pub notice_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingRefundRequest {
    pub id: String,
    pub state: String,
    pub reason_code: String,
    pub requested_at: String,
    #[serde(default)]
    pub decision_code: String,
    #[serde(default)]
    pub decision_mode: String,
    #[serde(default)]
    pub approved_amount_kopeks: i32,
    #[serde(default = "billing_refund_request_currency")]
    pub currency: String,
    #[serde(default = "billing_refund_request_renewal_stopped")]
    pub renewal_stopped: bool,
    #[serde(default)]
    pub current_access_ends_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingPlanChange {
    pub mode: String,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub target_offer_id: String,
    #[serde(default)]
    pub effective_at: String,
    #[serde(default)]
    pub carryover_limit: i32,
    #[serde(default)]
    pub client_checkout: Option<ConsumerCheckoutPresentation>,
}

fn billing_payment_method_state() -> String { "unknown".into() }
fn billing_payment_method_kind() -> String { "card".into() }
fn billing_catalog_currency() -> String { "RUB".into() }
fn billing_catalog_trial_offer() -> ConsumerBillingTrialOffer { ConsumerBillingTrialOffer::default() }
fn billing_catalog_starter_grant() -> ConsumerStarterGrantOffer { ConsumerStarterGrantOffer::default() }
fn billing_catalog_storefront() -> ConsumerBillingStorefront { ConsumerBillingStorefront::default() }
fn billing_catalog_promo_program() -> ConsumerPromoProgram { ConsumerPromoProgram::default() }
fn billing_status_subscription() -> ConsumerBillingSubscription { ConsumerBillingSubscription::default() }
fn billing_status_starter_grant() -> ConsumerStarterGrant { ConsumerStarterGrant::default() }
fn billing_subscription_state() -> String { "none".into() }
fn billing_refund_request_currency() -> String { "RUB".into() }
fn billing_refund_request_renewal_stopped() -> bool { true }
