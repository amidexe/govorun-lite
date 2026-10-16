//! Typed consumer views of admitted Core views; no transport or authority.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerTBankCheckoutPreview {
    #[serde(default)]
    pub client_checkout_preview: Option<ConsumerCheckoutPreviewAction>,
    #[serde(default = "t_bank_checkout_preview_payment_method_kind")]
    pub payment_method_kind: String,
    #[serde(default = "t_bank_checkout_preview_available_payment_methods")]
    pub available_payment_methods: Vec<String>,
    #[serde(default = "t_bank_checkout_preview_checkout_available")]
    pub checkout_available: bool,
    #[serde(default = "t_bank_checkout_preview_catalog_version")]
    pub catalog_version: String,
    pub plan_code: String,
    #[serde(default)]
    pub offer_id: String,
    pub purchase_kind: String,
    pub trial_days: i32,
    pub free_access_source: String,
    pub free_access_ends_at: String,
    pub first_charge_at: String,
    pub price_kopeks: i32,
    pub period_days: i32,
    pub currency: String,
    pub server_confirmation_required: bool,
    pub receipt_contact_state: String,
    #[serde(default)]
    pub receipt_email_masked: String,
    #[serde(default)]
    pub receipt_email: String,
    #[serde(default)]
    pub receipt_contact_token: String,
    pub amount_due_now_kopeks: i32,
    pub first_charge_amount_kopeks: i32,
    pub renewal_charge_at: String,
    pub renewal_amount_kopeks: i32,
    pub automatic_renewal: bool,
    pub cancellation_method: String,
    pub subscription_consent_initially_accepted: bool,
    pub subscription_consent_token: String,
    pub legal_documents: ConsumerTBankLegalDocuments,
    #[serde(default)]
    pub promotion: Option<ConsumerPromoOffer>,
    #[serde(default)]
    pub from_plan_code: String,
    #[serde(default)]
    pub checkout_intent_token: String,
    #[serde(default)]
    pub service_period_ends_at: String,
    #[serde(default)]
    pub first_charge_delay_seconds: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerCheckoutPreviewAction {
    pub can_confirm: bool,
    pub confirmation_path: String,
    pub receipt_contact_required: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerTBankLegalDocument {
    pub version: String,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerTBankLegalDocuments {
    pub subscription_terms: ConsumerTBankLegalDocument,
    pub user_agreement: ConsumerTBankLegalDocument,
    pub refund_policy: ConsumerTBankLegalDocument,
    pub privacy_notice: ConsumerTBankLegalDocument,
    pub receipt_data_consent: ConsumerTBankLegalDocument,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerTBankCheckoutSession {
    #[serde(default = "t_bank_checkout_session_payment_method_kind")]
    pub payment_method_kind: String,
    #[serde(default = "t_bank_checkout_session_catalog_version")]
    pub catalog_version: String,
    pub checkout_session_id: String,
    pub state: String,
    pub plan_code: String,
    #[serde(default)]
    pub offer_id: String,
    pub purchase_kind: String,
    pub trial_days: i32,
    pub free_access_source: String,
    pub free_access_ends_at: String,
    pub first_charge_at: String,
    pub price_kopeks: i32,
    pub period_days: i32,
    pub currency: String,
    pub browser_return_grants_access: bool,
    pub server_confirmation_required: bool,
    #[serde(default)]
    pub payment_url: String,
    #[serde(default)]
    pub first_charge_delay_seconds: Option<i32>,
    #[serde(default)]
    pub can_restart_binding: bool,
    #[serde(default)]
    pub cancellation_pending: bool,
    #[serde(default)]
    pub expires_in_seconds: Option<i32>,
    #[serde(default)]
    pub client_checkout: Option<ConsumerCheckoutPresentation>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerCheckoutPresentation {
    pub disposition: String,
    pub can_cancel: bool,
    pub resume_url: Option<String>,
    pub expires_in_seconds: Option<i32>,
    pub message: String,
    #[serde(default)]
    pub access_refresh_pending: bool,
    #[serde(default)]
    pub launch_action: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerCheckoutMessageUpdate {
    #[serde(default)]
    pub replace: bool,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub refresh_status: bool,
    #[serde(default)]
    pub refresh_access: bool,
}
impl Default for ConsumerCheckoutMessageUpdate {
    fn default() -> Self { Self { replace: false, message: None, refresh_status: false, refresh_access: false } }
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConsumerBillingCheckoutPresentation {
    #[serde(default = "billing_checkout_presentation_settled")]
    pub settled: ConsumerCheckoutMessageUpdate,
    #[serde(default = "billing_checkout_presentation_scheduled")]
    pub scheduled: ConsumerCheckoutMessageUpdate,
}
impl Default for ConsumerBillingCheckoutPresentation {
    fn default() -> Self { Self { settled: ConsumerCheckoutMessageUpdate::default(), scheduled: ConsumerCheckoutMessageUpdate { refresh_status: true, ..Default::default() } } }
}

fn t_bank_checkout_preview_payment_method_kind() -> String { "card".into() }
fn t_bank_checkout_preview_available_payment_methods() -> Vec<String> { vec!["card".into()] }
fn t_bank_checkout_preview_checkout_available() -> bool { true }
fn t_bank_checkout_preview_catalog_version() -> String { "test".into() }
fn t_bank_checkout_session_payment_method_kind() -> String { "card".into() }
fn t_bank_checkout_session_catalog_version() -> String { "test".into() }
fn billing_checkout_presentation_settled() -> ConsumerCheckoutMessageUpdate { ConsumerCheckoutMessageUpdate::default() }
fn billing_checkout_presentation_scheduled() -> ConsumerCheckoutMessageUpdate { ConsumerCheckoutMessageUpdate { refresh_status: true, ..Default::default() } }
