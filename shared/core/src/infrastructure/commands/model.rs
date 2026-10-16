use serde::Deserialize;
use super::server_input::{ServerInput, Catalog, CheckoutSession, CheckoutPreview, PromotionOffer, PromotionCatalog};

/// Typed boundary for platform clients. No command can create a payment,
/// grant access or bypass the account's explicitly confirmed mutation owner.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum BusinessCommand {
    ConsumerView { kind: crate::consumer_models::Kind, value: serde_json::Value, #[serde(default)] version: String },
    AuthHandoff { subscription_at: i64, stylization_at: i64, now: i64 },
    UsageBalanceCodec { command: serde_json::Value },
    LocalModelCodec { command: serde_json::Value },
    OptionalNotifications { command: serde_json::Value },
    AndroidDiagnostics { command: serde_json::Value },
    AndroidStylePreferences { command: serde_json::Value },
    AndroidSelfHostedSettings { command: serde_json::Value },
    AccountDocumentRequest { command: serde_json::Value },
    AndroidStyleVault { vault: serde_json::Value },
    StyleScope { command: crate::style_scope::Command },
    AccountStyleRequest { command: crate::style_pass_vault::account_wire::Command },
    StyleGatewayRequest { command: crate::stylization::request_wire::Command },
    StylePassCrypto {
        command: crate::style_pass_crypto_wire::Command,
    },
    SettingsAction {
        command: crate::settings_actions::Command,
    },
    ContentWorkflow {
        command: crate::content_workflow::Command,
    },
    StyleLibrary {
        command: crate::style_library::Command,
    },
    Synchronization {
        command: crate::synchronization::Command,
    },
    StatisticsLedger {
        command: crate::statistics_ledger::Command,
    },
    PreferencesSync {
        command: crate::preference_sync::Command,
    },
    StatisticsSync {
        command: crate::statistics_sync::Command,
    },
    Dictionary {
        command: crate::dictionary::Command,
    },
    ContentSync {
        command: crate::content_sync::Command,
    },
    DictationPreflight {
        command: crate::dictation_plan::Command,
    },
    StyleJobPolling {
        state: Option<crate::style_poll::State>,
        event: crate::style_poll::Event,
    },
    StyleJobResponse {
        phase: crate::style_job_response::Phase,
        body: String,
    },
    StyleStartAdmission {
        state: Option<crate::style_start::State>,
        event: crate::style_start::Event,
    },
    StyleTextAdmission {
        units: Vec<u16>,
    },
    CreditLease {
        state: Option<crate::credit_lease::State>,
        observations: Vec<crate::credit_lease::Observation>,
        epoch: u64,
        now_millis: u64,
        grace_millis: Option<u64>,
        allowed: bool,
    },
    ReconnectRecovery {
        facts: crate::reconnect_recovery::Facts,
    },
    CreditRecoveryRequest {
        passes: Vec<String>,
    },
    CreditRecoveryDelay {
        milliseconds: Option<i64>,
    },
    CreditRecoveryWire {
        status: Option<u16>,
        body: String,
    },
    CreditRetirementWire {
        status: Option<u16>,
        body: String,
        expected_count: usize,
        retry_after: Option<String>,
        #[serde(default)]
        now_millis: i64,
    },
    StyleReserveBinding {
        facts: crate::style_reserve_binding::Facts,
    },
    CreditRecovery {
        state: Option<crate::credit_recovery::State>,
        event: crate::credit_recovery::Event,
        epoch: u64,
        now_millis: u64,
        allowed: bool,
    },
    ReserveMaintenance {
        state: Option<crate::reserve_maintenance::State>,
        event: crate::reserve_maintenance::Event,
        epoch: u64,
        now_millis: u64,
        allowed: bool,
    },
    PrepareStyleIssuance {
        preparation: crate::style_issuance::Preparation,
    },
    StyleIssuanceJournal {
        event: crate::style_issuance_journal::Event,
    },
    StyleInventory {
        state: crate::style_inventory::State,
        event: crate::style_inventory::Event,
        now_millis: u64,
        retirement_available: bool,
    },
    StyleIssueFailure {
        status: Option<u16>,
        body: crate::style_issue::FailureBody,
    },
    StyleUsageLedger {
        ledger: crate::usage_ledger::Ledger,
        event: crate::usage_ledger::Event,
    },
    StyleReservePlan {
        facts: crate::style_window::StyleWindowFacts,
        context: crate::style_refill::Context,
    },
    StylePreparation {
        facts: crate::style_window::StyleWindowFacts,
        context: crate::style_preparation::Context,
    },
    StyleWindowPresentation {
        facts: crate::style_window::StyleWindowFacts,
    },
    AccessInteraction {
        facts: crate::access_interaction::AccessInteractionFacts,
        action: crate::access_interaction::Action,
        local_terms: bool,
        confirmed: bool,
        pending: bool,
    },
    StyleAccessPresentation {
        facts: crate::style_activation::Facts,
    },
    StyleActivation {
        facts: crate::style_activation::Facts,
        event: crate::style_activation::Event,
    },
    AccessPresentation {
        profile: Option<ServerInput<crate::Profile>>,
        client_terms: u32,
        signed_in: bool,
        fresh: bool,
        storage_available: bool,
    },
    SubscriptionInteraction {
        action: crate::subscription_interaction::Action,
        access: crate::subscription_interaction::Access,
        accepted: bool,
        handoff_pending: bool,
    },
    SubscriptionOverview {
        facts: crate::subscription::overview::OverviewFacts,
    },
    BillingLaunchFailure,
    BillingManagementVisible { subscription: serde_json::Value },
    BillingPaymentMethodView { method: serde_json::Value, status_available: bool },
    StyleReserveTransition {
        previous: Option<crate::style_reserve::Projection>,
        current: crate::style_reserve::Projection,
    },
    ProfileRefreshActivity {
        state: Option<crate::ProfileActivity>,
        event: crate::profile_activity::ProfileActivityEvent,
        epoch: Option<u64>,
        now_millis: u64,
        jitter_millis: Option<u64>,
    },
    SamePublishedCatalog {
        first: crate::checkout::CatalogPublication,
        second: crate::checkout::CatalogPublication,
    },
    CheckoutSession {
        session: ServerInput<CheckoutSession>,
    },
    CheckoutPreview {
        preview: ServerInput<CheckoutPreview>,
    },
    BillingCatalog {
        catalog: ServerInput<Catalog>,
        channel: crate::CatalogChannel,
    },
    PromotionFailure {
        status: Option<u16>,
        error: Option<String>,
    },
    NormalizePromotion {
        code: String,
    },
    ResolvePromotionOfferHint {
        requested_offer: String,
        available_offers: Vec<String>,
    },
    InspectPromotionOffer {
        offer: ServerInput<PromotionOffer>,
    },
    ValidatePromotionOffer {
        offer: ServerInput<PromotionOffer>,
        catalog: Option<PromotionCatalog>,
        requested_offer: Option<String>,
    },
}
