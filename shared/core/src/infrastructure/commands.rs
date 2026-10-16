#[path = "commands/server_input.rs"] mod server_input;
use crate::{promotions, Error, Result};
use serde_json::{json, Value};

#[path="commands/model.rs"]
mod model;
use model::BusinessCommand;

fn evaluate(command: BusinessCommand) -> Result<Value> {
    match command {
        BusinessCommand::ConsumerView { kind, value, version } => crate::consumer_models::evaluate(kind, value, version),
        BusinessCommand::AuthHandoff { subscription_at,stylization_at,now } => Ok(serde_json::json!({"destination":crate::oauth_flow::handoff_destination(subscription_at,stylization_at,now)})),
        BusinessCommand::UsageBalanceCodec { command } => crate::usage_ledger::balance_binding::evaluate(command),
        BusinessCommand::LocalModelCodec { command } => crate::local_model_codec::evaluate(command),
        BusinessCommand::OptionalNotifications { command } => Ok(json!({"value":crate::notifications::evaluate(command)?})),
        BusinessCommand::AndroidDiagnostics { command } => crate::operational_trace::android::evaluate(command),
        BusinessCommand::AndroidStylePreferences { command } => crate::style_library::android_preferences::evaluate(command),
        BusinessCommand::AndroidSelfHostedSettings { command } => crate::self_hosted_flow::android_settings::evaluate(command),
        BusinessCommand::AccountDocumentRequest { command } => crate::account_protocol::request_wire::evaluate(command),
        BusinessCommand::AndroidStyleVault { vault } => crate::style_pass_vault::android_v8::validate(vault),
        BusinessCommand::StyleScope { command } => crate::style_scope::evaluate(command),
        BusinessCommand::AccountStyleRequest { command } => crate::style_pass_vault::account_wire::evaluate(command),
        BusinessCommand::StyleGatewayRequest { command } => crate::stylization::request_wire::evaluate(command),
        BusinessCommand::StylePassCrypto { command } => {
            crate::style_pass_crypto_wire::execute(command).map_err(Error::Crypto)
        }
        BusinessCommand::SettingsAction { command } => crate::settings_actions::evaluate(command),
        BusinessCommand::ContentWorkflow { command } => crate::content_workflow::evaluate(command),
        BusinessCommand::StyleLibrary { command } => crate::style_library::evaluate(command),
        BusinessCommand::PreferencesSync { command } => crate::preference_sync::evaluate(command),
        BusinessCommand::StatisticsSync { command } => crate::statistics_sync::evaluate(command),
        BusinessCommand::Synchronization { command } => crate::synchronization::evaluate(command),
        BusinessCommand::StatisticsLedger { command } => {
            crate::statistics_ledger::evaluate(command)
        }
        BusinessCommand::Dictionary { command } => crate::dictionary::evaluate(command),
        BusinessCommand::ContentSync { command } => crate::content_sync::evaluate(command),
        BusinessCommand::DictationPreflight { command } => crate::dictation_plan::evaluate(command),
        BusinessCommand::StyleJobPolling { state, event } => {
            crate::style_poll::transition(state, event)
        }
        BusinessCommand::StyleJobResponse { phase, body } => {
            serde_json::to_value(crate::style_job_response::decide(phase, &body))
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleStartAdmission { state, event } => {
            crate::style_start::transition(state, event)
        }
        BusinessCommand::StyleTextAdmission { units } => crate::style_text::admit(units),
        BusinessCommand::CreditLease {
            state,
            observations,
            epoch,
            now_millis,
            grace_millis,
            allowed,
        } => serde_json::to_value(crate::credit_lease::transition(
            state,
            observations,
            epoch,
            now_millis,
            grace_millis,
            allowed,
        )?)
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::ReconnectRecovery { facts } => {
            Ok(json!({"action":crate::reconnect_recovery::action(facts)}))
        }
        BusinessCommand::CreditRecoveryRequest { passes } => {
            crate::credit_recovery_wire::request(passes)
        }
        BusinessCommand::CreditRecoveryDelay { milliseconds } => {
            Ok(json!({"milliseconds":crate::credit_recovery_wire::bounded_delay(milliseconds)}))
        }
        BusinessCommand::CreditRecoveryWire { status, body } => {
            serde_json::to_value(crate::credit_recovery_wire::decide(status, &body))
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::CreditRecovery {
            state,
            event,
            epoch,
            now_millis,
            allowed,
        } => serde_json::to_value(crate::credit_recovery::transition(
            state, event, epoch, now_millis, allowed,
        )?)
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::ReserveMaintenance {
            state,
            event,
            epoch,
            now_millis,
            allowed,
        } => serde_json::to_value(crate::reserve_maintenance::transition(
            state, event, epoch, now_millis, allowed,
        )?)
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::StyleIssuanceJournal { event } => {
            crate::style_issuance_journal::evaluate(event)
        }
        BusinessCommand::CreditRetirementWire {
            status,
            body,
            expected_count,
            retry_after,
            now_millis,
        } => crate::credit_retirement_wire::decide(
            status,
            &body,
            expected_count,
            retry_after.as_deref(),
            now_millis,
        ),
        BusinessCommand::StyleReserveBinding { facts } => {
            Ok(crate::style_reserve_binding::decide(facts))
        }
        BusinessCommand::StyleInventory {
            state,
            event,
            now_millis,
            retirement_available,
        } => serde_json::to_value(crate::style_inventory::transition(
            state,
            event,
            now_millis,
            retirement_available,
        )?)
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::PrepareStyleIssuance { preparation } => {
            serde_json::to_value(crate::style_issuance::prepare(preparation)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleIssueFailure { status, body } => {
            serde_json::to_value(crate::style_issue::failure(status, body)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleUsageLedger { ledger, event } => {
            serde_json::to_value(crate::usage_ledger::apply(ledger, event)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleReservePlan { facts, context } => {
            serde_json::to_value(crate::style_refill::plan(facts, context)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StylePreparation { facts, context } => {
            serde_json::to_value(crate::style_preparation::plan(facts, context)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleWindowPresentation { facts } => {
            serde_json::to_value(crate::style_window::project(facts)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::AccessInteraction {
            facts,
            action,
            local_terms,
            confirmed,
            pending,
        } => serde_json::to_value(crate::access_interaction::plan(
            facts,
            action,
            local_terms,
            confirmed,
            pending,
        )?)
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::StyleAccessPresentation { facts } => {
            serde_json::to_value(crate::style_activation::project(&facts)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::StyleActivation { facts, event } => {
            serde_json::to_value(crate::style_activation::transition(facts, event)?)
                .map_err(|_| Error::InvalidResponse)
        }
        BusinessCommand::AccessPresentation {
            profile,
            client_terms,
            signed_in,
            fresh,
            storage_available,
        } => {
            if client_terms == 0 {
                return Err(Error::InvalidRequest);
            }
            let profile = profile.map(server_input::ServerInput::resolve).transpose()?;
            Ok(serde_json::to_value(crate::AccessPresentation::project(
                profile.as_ref(),
                client_terms,
                signed_in,
                fresh,
                storage_available,
            ))
            .map_err(|_| Error::InvalidResponse)?)
        }
        BusinessCommand::SubscriptionInteraction {
            action,
            access,
            accepted,
            handoff_pending,
        } => crate::subscription_interaction::interaction(
            action,
            &access,
            accepted,
            handoff_pending,
        ),
        BusinessCommand::SubscriptionOverview { facts } => serde_json::to_value(
            crate::subscription::overview::SubscriptionOverview::project(facts)?,
        )
        .map_err(|_| Error::InvalidResponse),
        BusinessCommand::BillingPaymentMethodView { method, status_available } =>
            Ok(crate::features::billing::snapshot::payment_method_view(&method, status_available)),
        BusinessCommand::BillingManagementVisible { subscription } =>
            Ok(json!({"visible": crate::billing_status::management_visible(&subscription)})),
        BusinessCommand::BillingLaunchFailure => Ok(json!({"message":"Не удалось открыть оплату"})),
        BusinessCommand::StyleReserveTransition { previous, current } => {
            crate::style_reserve::transition(previous, current)
        }
        BusinessCommand::ProfileRefreshActivity {
            state,
            event,
            epoch,
            now_millis,
            jitter_millis,
        } => crate::profile_activity::evaluate(
            state.unwrap_or_default(),
            event,
            epoch,
            now_millis,
            jitter_millis,
        ),
        BusinessCommand::SamePublishedCatalog { first, second } => Ok(json!({
            "same": first == second,
        })),
        BusinessCommand::CheckoutSession { session } => Ok(session.resolve()?.projection()),
        BusinessCommand::CheckoutPreview { preview } => {
            Ok(json!({"preview": preview.resolve()?.projection()}))
        }
        BusinessCommand::BillingCatalog { catalog, channel } => Ok(json!({
            "catalog": catalog.resolve()?.project(channel)?,
        })),
        BusinessCommand::PromotionFailure { status, error } => Ok(json!({
            "message": promotions::failure_message(status, error.as_deref()),
        })),
        BusinessCommand::NormalizePromotion { code } => Ok(json!({
            "code": promotions::normalize_code(&code)?,
        })),
        BusinessCommand::ResolvePromotionOfferHint {
            requested_offer,
            available_offers,
        } => Ok(json!({
            "offerId": if available_offers.contains(&requested_offer) { requested_offer } else { String::new() },
        })),
        BusinessCommand::InspectPromotionOffer { offer } => Ok(json!({
            "requiresCatalog": offer.resolve()?.requires_catalog(),
        })),
        BusinessCommand::ValidatePromotionOffer {
            offer,
            catalog,
            requested_offer,
        } => {
            let offer = offer.resolve()?;
            offer.validate(catalog.as_ref(), requested_offer.as_deref())?;
            Ok(json!({}))
        }
    }
}

pub fn evaluate_business_command(input: &str) -> String {
    // Journals and local transcript rules share the bounded native envelope.
    // Account/network commands retain the smaller limit. Local text is never sent
    // to an account service merely to validate a user-owned server response.
    let result = if input.len() > 8 * 1024 * 1024 {
        Err(Error::ResponseTooLarge)
    } else {
        serde_json::from_str(input)
            .map_err(|_| {
                if input.len() > 524_288 {
                    Error::ResponseTooLarge
                } else {
                    Error::InvalidRequest
                }
            })
            .and_then(|command| {
                let large_local_payload = matches!(
                    &command,
                    BusinessCommand::ContentWorkflow { .. }
                        | BusinessCommand::AndroidSelfHostedSettings { .. }
                        | BusinessCommand::AndroidStylePreferences { .. }
                        | BusinessCommand::AndroidStyleVault { .. }
                        | BusinessCommand::StatisticsLedger { .. }
                        | BusinessCommand::PreferencesSync { .. }
                        | BusinessCommand::StatisticsSync { .. }
                        | BusinessCommand::Dictionary { .. }
                        | BusinessCommand::ContentSync { .. }
                        | BusinessCommand::StyleUsageLedger { .. }
                        | BusinessCommand::DictationPreflight {
                            command: crate::dictation_plan::Command::SelfHosted { .. }
                                | crate::dictation_plan::Command::Delivery { .. }
                        }
                );
                if input.len() > 524_288 && !large_local_payload {
                    Err(Error::ResponseTooLarge)
                } else {
                    evaluate(command)
                }
            })
    };
    match result {
        Ok(value) => json!({"result": value}),
        Err(error) => json!({"error": error.code(), "status": match error {
            Error::Http(status) => Some(status), _ => None,
        }}),
    }
    .to_string()
}

/// Build-only schema for the complete command boundary; no production generator.
#[cfg(feature = "schema")]
pub fn command_schema() -> schemars::schema::RootSchema {
    schemars::schema_for!(model::BusinessCommand)
}
