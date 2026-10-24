//! Shared rules and feature lifetimes for Android, macOS and Windows.
extern crate self as govorun_core;
pub mod features;
/// Shared agent models, rules and port-facing operations.
#[path="infrastructure/agent/mod.rs"]
pub mod agent;
/// Shared account storage models, rules and port-facing operations.
#[path="account/account_storage.rs"]
pub mod account_storage;
/// Shared account host models, rules and port-facing operations.
#[path="account/account_host.rs"]
pub mod account_host;
/// Shared access models, rules and port-facing operations.
#[path="account/access.rs"]
mod access;
/// Shared access interaction models, rules and port-facing operations.
#[path="account/access_interaction.rs"]
mod access_interaction;
/// Shared access offer models, rules and port-facing operations.
#[path="account/access_offer.rs"]
mod access_offer;
/// Shared access presentation models, rules and port-facing operations.
#[path="account/access_presentation.rs"]
mod access_presentation;
/// Shared account protocol models, rules and port-facing operations.
#[path="account/account_protocol/mod.rs"]
pub mod account_protocol;
/// Shared activation models, rules and port-facing operations.
#[path="account/activation.rs"]
mod activation;
/// Shared activity models, rules and port-facing operations.
#[path="account/activity.rs"]
mod activity;
/// Shared auth session models, rules and port-facing operations.
#[path="account/auth_session.rs"]
mod auth_session;
/// Shared access models, rules and operations through ports.
#[path="account/authority.rs"]
mod authority;
/// Shared benchmark report models, rules and port-facing operations.
#[path="diagnostics/benchmark_report/mod.rs"]
pub mod benchmark_report;
/// Shared billing actions models, rules and port-facing operations.
#[path="billing/billing_actions.rs"]
mod billing_actions;
pub use billing_actions::BillingActions;
/// Shared billing status models, rules and port-facing operations.
#[path="billing/billing_status.rs"]
mod billing_status;
/// Shared catalog models, rules and port-facing operations.
#[path="billing/catalog.rs"]
mod catalog;
/// Shared catalog presentation models, rules and port-facing operations.
#[path="billing/catalog_presentation.rs"]
mod catalog_presentation;
/// Shared catalog refresh models, rules and port-facing operations.
#[path="billing/catalog_refresh.rs"]
mod catalog_refresh;
/// Shared checkout models, rules and port-facing operations.
#[path="billing/checkout/mod.rs"]
mod checkout;
/// Shared clipboard settings models, rules and port-facing operations.
#[path="dictation/clipboard_settings.rs"]
pub mod clipboard_settings;
/// Shared commands models, rules and port-facing operations.
#[path="infrastructure/commands.rs"]
mod commands;
/// Shared configuration models, rules and port-facing operations.
#[path="infrastructure/configuration.rs"]
mod configuration;
/// Shared consent models, rules and port-facing operations.
#[path="account/consent.rs"]
mod consent;
/// Shared content sync models, rules and port-facing operations.
#[path="sync/content_sync.rs"]
mod content_sync;
/// Shared content workflow models, rules and port-facing operations.
#[path="sync/content_workflow.rs"]
mod content_workflow;
/// Shared credit lease models, rules and port-facing operations.
#[path="credits/credit_lease.rs"]
mod credit_lease;
/// Shared credit recovery models, rules and port-facing operations.
#[path="credits/credit_recovery.rs"]
mod credit_recovery;
/// Shared credit recovery wire models, rules and port-facing operations.
#[path="credits/credit_recovery_wire.rs"]
mod credit_recovery_wire;
/// Shared credit retirement wire models, rules and port-facing operations.
#[path="credits/credit_retirement_wire.rs"]
mod credit_retirement_wire;
/// Shared devices models, rules and port-facing operations.
#[path="account/devices.rs"]
mod devices;
/// Shared dictation delivery models, rules and port-facing operations.
#[path="dictation/dictation_delivery.rs"]
mod dictation_delivery;
/// Shared dictation flow models, rules and port-facing operations.
#[path="dictation/dictation_flow.rs"]
mod dictation_flow;
/// Shared dictation access models, rules and port-facing operations.
#[path="dictation/dictation_access.rs"]
mod dictation_access;
/// Shared dictation plan models, rules and port-facing operations.
#[path="dictation/dictation_plan.rs"]
mod dictation_plan;
/// Shared dictation timing models, rules and port-facing operations.
#[path="dictation/dictation_timing.rs"]
mod dictation_timing;
/// Shared dictionary models, rules and port-facing operations.
#[path="dictionary/dictionary.rs"]
mod dictionary;
/// Shared dictionary document models, rules and port-facing operations.
#[path="dictionary/dictionary_document.rs"]
mod dictionary_document;
/// Shared dictionary replacement models, rules and port-facing operations.
#[path="dictionary/dictionary_replacement.rs"]
mod dictionary_replacement;
/// Shared dictionary repository models, rules and port-facing operations.
#[path="dictionary/dictionary_repository.rs"]
pub mod dictionary_repository;
/// Shared dictionary storage models, rules and port-facing operations.
#[path="dictionary/dictionary_storage.rs"]
mod dictionary_storage;
/// Shared dictionary voice models, rules and port-facing operations.
#[path="dictionary/dictionary_voice.rs"]
mod dictionary_voice;
/// Shared error models, rules and port-facing operations.
#[path="infrastructure/error.rs"]
mod error;
/// Shared hotkeys models, rules and port-facing operations.
#[path="dictation/hotkeys/mod.rs"]
pub mod hotkeys;
/// Shared journal report models, rules and port-facing operations.
#[path="diagnostics/journal_report/mod.rs"]
pub mod journal_report;
/// Shared lifecycle models, rules and port-facing operations.
#[path="account/lifecycle.rs"]
mod lifecycle;
/// Shared oauth models, rules and port-facing operations.
#[path="account/oauth.rs"]
mod oauth;
/// Shared oauth flow models, rules and port-facing operations.
#[path="account/oauth_flow.rs"]
mod oauth_flow;
/// Shared online outage models, rules and port-facing operations.
#[path="styles/online_outage.rs"]
mod online_outage;
/// Shared online protocol models, rules and port-facing operations.
#[path="styles/online_protocol.rs"]
mod online_protocol;
/// Shared operational trace models, rules and port-facing operations.
#[path="diagnostics/operational_trace.rs"]
pub mod operational_trace;
/// Shared permission setup models, rules and port-facing operations.
#[path="dictation/permission_setup/mod.rs"]
pub mod permission_setup;
/// Shared preference model models, rules and port-facing operations.
#[path="sync/preference_model.rs"]
mod preference_model;
/// Shared preference sync models, rules and port-facing operations.
#[path="sync/preference_sync.rs"]
mod preference_sync;
/// Shared profile models, rules and port-facing operations.
#[path="account/profile.rs"]
mod profile;
/// Shared profile activity models, rules and port-facing operations.
#[path="account/profile_activity.rs"]
mod profile_activity;
/// Shared profile cadence models, rules and port-facing operations.
#[path="account/profile_cadence.rs"]
mod profile_cadence;
/// Shared profile fetch models, rules and port-facing operations.
#[path="account/profile_fetch.rs"]
mod profile_fetch;
/// Shared profile refresh models, rules and port-facing operations.
#[path="account/profile_refresh.rs"]
mod profile_refresh;
/// Shared promotions models, rules and port-facing operations.
#[path="billing/promotions/mod.rs"]
mod promotions;
/// Shared reconnect recovery models, rules and port-facing operations.
#[path="credits/reconnect_recovery.rs"]
mod reconnect_recovery;
/// Shared recording controls models, rules and port-facing operations.
#[path="dictation/recording_controls.rs"]
pub mod recording_controls;
/// Shared dictation jobs models, rules and port-facing operations.
#[path="dictation/dictation_jobs.rs"]
pub mod dictation_jobs;
/// Shared recording plan models, rules and port-facing operations.
#[path="dictation/recording_plan.rs"]
mod recording_plan;
/// Shared recording support models, rules and port-facing operations.
#[path="dictation/recording_support.rs"]
pub mod recording_support;
/// Shared refresh models, rules and port-facing operations.
#[path="account/refresh.rs"]
mod refresh;
/// Shared request models, rules and port-facing operations.
#[path="infrastructure/request.rs"]
mod request;
/// Shared reserve maintenance models, rules and port-facing operations.
#[path="credits/reserve_maintenance.rs"]
mod reserve_maintenance;
/// Shared self hosted flow models, rules and port-facing operations.
#[path="dictation/self_hosted_flow.rs"]
pub mod self_hosted_flow;
/// Shared session models, rules and port-facing operations.
#[path="account/session.rs"]
mod session;
/// Shared session sign in models, rules and port-facing operations.
#[path="account/session_sign_in.rs"]
mod session_sign_in;
/// Shared settings actions models, rules and port-facing operations.
#[path="account/settings_actions.rs"]
mod settings_actions;
/// Shared sign in models, rules and port-facing operations.
#[path="account/sign_in.rs"]
mod sign_in;
/// Shared sign in commit models, rules and port-facing operations.
#[path="account/sign_in_commit.rs"]
mod sign_in_commit;
/// Shared statistics continuity models, rules and port-facing operations.
#[path="statistics/statistics_continuity.rs"]
mod statistics_continuity;
/// Shared statistics ledger models, rules and port-facing operations.
#[path="statistics/statistics_ledger.rs"]
mod statistics_ledger;
/// Shared statistics ledger model models, rules and port-facing operations.
#[path="statistics/statistics_ledger_model.rs"]
mod statistics_ledger_model;
/// Shared statistics recovery models, rules and port-facing operations.
#[path="statistics/statistics_recovery.rs"]
mod statistics_recovery;
/// Shared statistics repository models, rules and port-facing operations.
#[path="statistics/statistics_repository.rs"]
pub mod statistics_repository;
/// Shared statistics sync models, rules and port-facing operations.
#[path="statistics/statistics_sync.rs"]
mod statistics_sync;
/// Shared storage models, rules and port-facing operations.
#[path="account/storage.rs"]
mod storage;
/// Shared store ports models, rules and port-facing operations.
#[path="infrastructure/store_ports.rs"]
mod store_ports;
/// Shared style activation models, rules and port-facing operations.
#[path="styles/style_activation.rs"]
mod style_activation;
/// Shared style definition models, rules and port-facing operations.
#[path="styles/style_definition.rs"]
mod style_definition;
/// Shared style display models, rules and port-facing operations.
#[path="styles/style_display.rs"]
mod style_display;
/// Shared style inventory models, rules and port-facing operations.
#[path="credits/style_inventory/mod.rs"]
pub mod style_inventory;
/// Shared style issuance models, rules and port-facing operations.
#[path="credits/style_issuance.rs"]
mod style_issuance;
/// Shared style issuance journal models, rules and port-facing operations.
#[path="credits/style_issuance_journal.rs"]
mod style_issuance_journal;
/// Shared style issue models, rules and port-facing operations.
#[path="credits/style_issue.rs"]
mod style_issue;
/// Shared style job response models, rules and port-facing operations.
#[path="styles/style_job_response.rs"]
mod style_job_response;
/// Shared style library models, rules and port-facing operations.
#[path="styles/style_library.rs"]
mod style_library;
/// Shared style pass crypto models, rules and port-facing operations.
#[path="credits/style_pass_crypto.rs"]
pub mod style_pass_crypto;
/// Shared style pass crypto wire models, rules and port-facing operations.
#[path="credits/style_pass_crypto_wire.rs"]
pub mod style_pass_crypto_wire;
/// Shared style pass vault models, rules and port-facing operations.
#[path="credits/style_pass_vault/mod.rs"]
pub mod style_pass_vault;
/// Shared style poll models, rules and port-facing operations.
#[path="styles/style_poll.rs"]
mod style_poll;
/// Shared style preparation models, rules and port-facing operations.
#[path="styles/style_preparation.rs"]
mod style_preparation;
/// Shared style quota models, rules and port-facing operations.
#[path="styles/style_quota.rs"]
mod style_quota;
/// Shared style refill models, rules and port-facing operations.
#[path="credits/style_refill.rs"]
mod style_refill;
/// Shared style repository models, rules and port-facing operations.
#[path="styles/style_repository.rs"]
pub mod style_repository;
/// Shared style reserve models, rules and port-facing operations.
#[path="credits/style_reserve.rs"]
mod style_reserve;
#[path="credits/style_scope.rs"]
mod style_scope;
/// Shared style reserve binding models, rules and port-facing operations.
#[path="credits/style_reserve_binding.rs"]
mod style_reserve_binding;
/// Shared style start models, rules and port-facing operations.
#[path="styles/style_start.rs"]
mod style_start;
/// Shared style text models, rules and port-facing operations.
#[path="styles/style_text.rs"]
mod style_text;
/// Shared stylization models, rules and port-facing operations.
#[path="styles/stylization/mod.rs"]
pub mod stylization;
/// Shared style window models, rules and port-facing operations.
#[path="styles/style_window.rs"]
mod style_window;
/// Shared subscription models, rules and port-facing operations.
#[path="billing/subscription/mod.rs"]
mod subscription;
/// Shared subscription interaction models, rules and port-facing operations.
#[path="billing/subscription_interaction.rs"]
mod subscription_interaction;
/// Shared sync enrollment models, rules and port-facing operations.
#[path="sync/sync_enrollment.rs"]
pub mod sync_enrollment;
/// Shared sync journal models, rules and port-facing operations.
#[path="sync/sync_journal.rs"]
pub mod sync_journal;
/// Shared sync wire models, rules and port-facing operations.
#[path="sync/sync_wire.rs"]
pub mod sync_wire;
/// Shared synchronization models, rules and port-facing operations.
#[path="sync/synchronization.rs"]
mod synchronization;
/// Shared transcript flow models, rules and port-facing operations.
#[path="dictation/transcript_flow.rs"]
mod transcript_flow;
/// Shared transport models, rules and port-facing operations.
#[path="infrastructure/transport.rs"]
mod transport;
/// Shared update feed models, rules and port-facing operations.
#[path="updates/update_policy.rs"]
pub mod update_policy;
/// Shared update verification models, rules and port-facing operations.
/// Shared usage day refresh models, rules and port-facing operations.
#[path="credits/usage_day_refresh.rs"]
mod usage_day_refresh;
/// Shared usage ledger models, rules and port-facing operations.
#[path="credits/usage_ledger/mod.rs"]
pub mod usage_ledger;
/// Shared wire models, rules and port-facing operations.
#[path="infrastructure/wire.rs"]
mod wire;

pub use access::{AccountAccessHost, AccountAccessService};
pub use access_offer::{project_free_access, FreeAccessOffer};
pub use activation::FreeAccessActivation;
pub use auth_session::{AuthSession, AuthSessionSnapshot};
pub use authority::{Authority, StyleObservation, TermsRelation};
pub use billing_status::{
    validate_billing_status, BillingManagementCommand, BillingManagementSnapshot,
    BillingRefundReason, BillingStatusService, BillingStatusSnapshot, BillingStatusSource,
};
pub use catalog::{
    catalog_for_channel, load_catalog, validate_catalog, CatalogChannel, CatalogSource,
};
pub use catalog_refresh::{CatalogRefresh, CatalogSnapshot};
pub use checkout::{
    prepare_checkout, validate_preview as validate_checkout_preview, CheckoutFailure, CheckoutHost,
    CheckoutInteraction, CheckoutRecoveryFailure, CheckoutService, CheckoutSnapshot,
};
pub use commands::evaluate_business_command;
pub use configuration::{
    accounts_origin as validate_account_origin, configured_account_origin,
    configured_refresh_import_allowed,
};
pub use error::{Error, Result};
pub use oauth::{AuthAttempt, AuthConfig, AuthDevice, AuthProvider, AuthStorage};
pub use oauth_flow::{AuthFlow, AuthIntent};
pub use profile_activity::{ProfileActivity, ProfileAdmission, ProfileNextStep};
pub use profile_cadence::{
    ProfileRefreshCadence, ProfileRefreshReason, PROFILE_FOCUS_MAX_AGE, PROFILE_REFRESH_INTERVAL,
    PROFILE_REFRESH_JITTER, PROFILE_REFRESH_MAX_BACKOFF, PROFILE_RETRY_INTERVAL, PROFILE_SYNC_MAX_AGE,
};
pub use profile_fetch::{load_account_profile, ProfileIdentity, ProfileSource};
pub use profile_refresh::{
    refresh_account_profile, ProfileRefreshHost, ProfileUnavailable, ProfileUpdate,
};
pub use promotions::{
    PromotionAccess, PromotionAccessKind, PromotionController, PromotionEffect, PromotionHost,
    PromotionService,
};
pub use request::{AccountRequest, Method};
pub use session::{AccountSession, SessionSnapshot, SessionOptions};
pub use sign_in::{complete_sign_in, validate_sign_in_request, SignInHost};
pub use storage::{Storage, StorageRead, StoredSession};
pub use store_ports::{Document, DocumentStore, SecretStore, StoreError, StoreResult};
pub use transport::{HttpResponse, Transport};
pub use wire::{Credentials, Profile};

pub type Future<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

pub use access_presentation::AccessPresentation;

/// Typed access intents shared by every client boundary.
pub use access_interaction::Action as AccessInteractionAction;
/// Typed subscription intents; these do not authorize a payment.
pub use subscription_interaction::Action as SubscriptionInteractionAction;

/// Build-only description of the private, bounded command boundary.
#[cfg(feature = "schema")]
pub use commands::command_schema;

#[path="infrastructure/boundary_snapshots.rs"]
pub mod boundary_snapshots;

#[path="infrastructure/instance.rs"]
pub mod instance;

mod notifications;

#[path="infrastructure/local_model_codec.rs"]
mod local_model_codec;

#[path="infrastructure/consumer_models/mod.rs"]
pub mod consumer_models;

#[path="credits/limits.rs"]
mod credit_limits;
