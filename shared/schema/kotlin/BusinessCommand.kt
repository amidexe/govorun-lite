// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommand { val `operation`: String }
data class BusinessCommand_consumerView(
    val `kind`: BusinessCommandKind,
    val `value`: Any?,
    val `version`: String
) : BusinessCommand { override val `operation`: String = "consumerView" }
data class BusinessCommand_authHandoff(
    val `now`: Long,
    val `stylization_at`: Long,
    val `subscription_at`: Long
) : BusinessCommand { override val `operation`: String = "authHandoff" }
data class BusinessCommand_usageBalanceCodec(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "usageBalanceCodec" }
data class BusinessCommand_localModelCodec(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "localModelCodec" }
data class BusinessCommand_optionalNotifications(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "optionalNotifications" }
data class BusinessCommand_androidDiagnostics(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "androidDiagnostics" }
data class BusinessCommand_androidStylePreferences(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "androidStylePreferences" }
data class BusinessCommand_androidSelfHostedSettings(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "androidSelfHostedSettings" }
data class BusinessCommand_accountDocumentRequest(
    val `command`: Any?
) : BusinessCommand { override val `operation`: String = "accountDocumentRequest" }
data class BusinessCommand_androidStyleVault(
    val `vault`: Any?
) : BusinessCommand { override val `operation`: String = "androidStyleVault" }
data class BusinessCommand_styleScope(
    val `command`: BusinessCommandCommand
) : BusinessCommand { override val `operation`: String = "styleScope" }
data class BusinessCommand_accountStyleRequest(
    val `command`: BusinessCommandCommand2
) : BusinessCommand { override val `operation`: String = "accountStyleRequest" }
data class BusinessCommand_styleGatewayRequest(
    val `command`: BusinessCommandCommand3
) : BusinessCommand { override val `operation`: String = "styleGatewayRequest" }
data class BusinessCommand_stylePassCrypto(
    val `command`: BusinessCommandCommand4
) : BusinessCommand { override val `operation`: String = "stylePassCrypto" }
data class BusinessCommand_settingsAction(
    val `command`: BusinessCommandCommand5
) : BusinessCommand { override val `operation`: String = "settingsAction" }
data class BusinessCommand_contentWorkflow(
    val `command`: BusinessCommandCommand6
) : BusinessCommand { override val `operation`: String = "contentWorkflow" }
data class BusinessCommand_styleLibrary(
    val `command`: BusinessCommandCommand7
) : BusinessCommand { override val `operation`: String = "styleLibrary" }
data class BusinessCommand_synchronization(
    val `command`: BusinessCommandCommand8
) : BusinessCommand { override val `operation`: String = "synchronization" }
data class BusinessCommand_statisticsLedger(
    val `command`: BusinessCommandCommand9
) : BusinessCommand { override val `operation`: String = "statisticsLedger" }
data class BusinessCommand_preferencesSync(
    val `command`: BusinessCommandCommand10
) : BusinessCommand { override val `operation`: String = "preferencesSync" }
data class BusinessCommand_statisticsSync(
    val `command`: BusinessCommandCommand11
) : BusinessCommand { override val `operation`: String = "statisticsSync" }
data class BusinessCommand_dictionary(
    val `command`: BusinessCommandCommand12
) : BusinessCommand { override val `operation`: String = "dictionary" }
data class BusinessCommand_contentSync(
    val `command`: BusinessCommandCommand13
) : BusinessCommand { override val `operation`: String = "contentSync" }
data class BusinessCommand_dictationPreflight(
    val `command`: BusinessCommandCommand14
) : BusinessCommand { override val `operation`: String = "dictationPreflight" }
data class BusinessCommand_styleJobPolling(
    val `event`: BusinessCommandEvent7,
    val `state`: BusinessCommandState7?
) : BusinessCommand { override val `operation`: String = "styleJobPolling" }
data class BusinessCommand_styleJobResponse(
    val `body`: String,
    val `phase`: BusinessCommandPhase4
) : BusinessCommand { override val `operation`: String = "styleJobResponse" }
data class BusinessCommand_styleStartAdmission(
    val `event`: BusinessCommandEvent8,
    val `state`: BusinessCommandState8?
) : BusinessCommand { override val `operation`: String = "styleStartAdmission" }
data class BusinessCommand_styleTextAdmission(
    val `units`: List<Int>
) : BusinessCommand { override val `operation`: String = "styleTextAdmission" }
data class BusinessCommand_creditLease(
    val `allowed`: Boolean,
    val `epoch`: Long,
    val `grace_millis`: Long?,
    val `now_millis`: Long,
    val `observations`: List<BusinessCommandObservation>,
    val `state`: BusinessCommandState9?
) : BusinessCommand { override val `operation`: String = "creditLease" }
data class BusinessCommand_reconnectRecovery(
    val `facts`: BusinessCommandFacts3
) : BusinessCommand { override val `operation`: String = "reconnectRecovery" }
data class BusinessCommand_creditRecoveryRequest(
    val `passes`: List<String>
) : BusinessCommand { override val `operation`: String = "creditRecoveryRequest" }
data class BusinessCommand_creditRecoveryDelay(
    val `milliseconds`: Long?
) : BusinessCommand { override val `operation`: String = "creditRecoveryDelay" }
data class BusinessCommand_creditRecoveryWire(
    val `body`: String,
    val `status`: Int?
) : BusinessCommand { override val `operation`: String = "creditRecoveryWire" }
data class BusinessCommand_creditRetirementWire(
    val `body`: String,
    val `expected_count`: Int,
    val `now_millis`: Long,
    val `retry_after`: String?,
    val `status`: Int?
) : BusinessCommand { override val `operation`: String = "creditRetirementWire" }
data class BusinessCommand_styleReserveBinding(
    val `facts`: BusinessCommandFacts4
) : BusinessCommand { override val `operation`: String = "styleReserveBinding" }
data class BusinessCommand_creditRecovery(
    val `allowed`: Boolean,
    val `epoch`: Long,
    val `event`: BusinessCommandEvent10,
    val `now_millis`: Long,
    val `state`: BusinessCommandState10?
) : BusinessCommand { override val `operation`: String = "creditRecovery" }
data class BusinessCommand_reserveMaintenance(
    val `allowed`: Boolean,
    val `epoch`: Long,
    val `event`: BusinessCommandEvent11,
    val `now_millis`: Long,
    val `state`: BusinessCommandState11?
) : BusinessCommand { override val `operation`: String = "reserveMaintenance" }
data class BusinessCommand_prepareStyleIssuance(
    val `preparation`: BusinessCommandPreparation
) : BusinessCommand { override val `operation`: String = "prepareStyleIssuance" }
data class BusinessCommand_styleIssuanceJournal(
    val `event`: BusinessCommandEvent12
) : BusinessCommand { override val `operation`: String = "styleIssuanceJournal" }
data class BusinessCommand_styleInventory(
    val `event`: BusinessCommandEvent13,
    val `now_millis`: Long,
    val `retirement_available`: Boolean,
    val `state`: BusinessCommandState12
) : BusinessCommand { override val `operation`: String = "styleInventory" }
data class BusinessCommand_styleIssueFailure(
    val `body`: BusinessCommandIssueFailureBody,
    val `status`: Int?
) : BusinessCommand { override val `operation`: String = "styleIssueFailure" }
data class BusinessCommand_styleUsageLedger(
    val `event`: BusinessCommandEvent14,
    val `ledger`: BusinessCommandLedger
) : BusinessCommand { override val `operation`: String = "styleUsageLedger" }
data class BusinessCommand_styleReservePlan(
    val `context`: BusinessCommandContext2,
    val `facts`: BusinessCommandStyleWindowFacts
) : BusinessCommand { override val `operation`: String = "styleReservePlan" }
data class BusinessCommand_stylePreparation(
    val `context`: BusinessCommandContext3,
    val `facts`: BusinessCommandStyleWindowFacts
) : BusinessCommand { override val `operation`: String = "stylePreparation" }
data class BusinessCommand_styleWindowPresentation(
    val `facts`: BusinessCommandStyleWindowFacts
) : BusinessCommand { override val `operation`: String = "styleWindowPresentation" }
data class BusinessCommand_accessInteraction(
    val `action`: BusinessCommandAccessInteractionAction,
    val `confirmed`: Boolean,
    val `facts`: BusinessCommandAccessInteractionFacts,
    val `local_terms`: Boolean,
    val `pending`: Boolean
) : BusinessCommand { override val `operation`: String = "accessInteraction" }
data class BusinessCommand_styleAccessPresentation(
    val `facts`: BusinessCommandFacts5
) : BusinessCommand { override val `operation`: String = "styleAccessPresentation" }
data class BusinessCommand_styleActivation(
    val `event`: BusinessCommandEvent15,
    val `facts`: BusinessCommandFacts5
) : BusinessCommand { override val `operation`: String = "styleActivation" }
data class BusinessCommand_accessPresentation(
    val `client_terms`: Int,
    val `fresh`: Boolean,
    val `profile`: BusinessCommandServerInput_Profile?,
    val `signed_in`: Boolean,
    val `storage_available`: Boolean
) : BusinessCommand { override val `operation`: String = "accessPresentation" }
data class BusinessCommand_subscriptionInteraction(
    val `accepted`: Boolean,
    val `access`: BusinessCommandAccess2,
    val `action`: BusinessCommandSubscriptionInteractionAction,
    val `handoff_pending`: Boolean
) : BusinessCommand { override val `operation`: String = "subscriptionInteraction" }
data class BusinessCommand_subscriptionOverview(
    val `facts`: BusinessCommandOverviewFacts
) : BusinessCommand { override val `operation`: String = "subscriptionOverview" }
data object BusinessCommand_billingLaunchFailure : BusinessCommand { override val `operation`: String = "billingLaunchFailure" }
data class BusinessCommand_billingManagementVisible(
    val `subscription`: Any?
) : BusinessCommand { override val `operation`: String = "billingManagementVisible" }
data class BusinessCommand_billingPaymentMethodView(
    val `method`: Any?,
    val `status_available`: Boolean
) : BusinessCommand { override val `operation`: String = "billingPaymentMethodView" }
data class BusinessCommand_styleReserveTransition(
    val `current`: BusinessCommandProjection,
    val `previous`: BusinessCommandProjection?
) : BusinessCommand { override val `operation`: String = "styleReserveTransition" }
data class BusinessCommand_profileRefreshActivity(
    val `epoch`: Long?,
    val `event`: BusinessCommandProfileActivityEvent,
    val `jitter_millis`: Long?,
    val `now_millis`: Long,
    val `state`: BusinessCommandProfileActivity?
) : BusinessCommand { override val `operation`: String = "profileRefreshActivity" }
data class BusinessCommand_samePublishedCatalog(
    val `first`: BusinessCommandCatalogPublication,
    val `second`: BusinessCommandCatalogPublication
) : BusinessCommand { override val `operation`: String = "samePublishedCatalog" }
data class BusinessCommand_checkoutSession(
    val `session`: BusinessCommandServerInput_CheckoutSession
) : BusinessCommand { override val `operation`: String = "checkoutSession" }
data class BusinessCommand_checkoutPreview(
    val `preview`: BusinessCommandServerInput_CheckoutPreview
) : BusinessCommand { override val `operation`: String = "checkoutPreview" }
data class BusinessCommand_billingCatalog(
    val `catalog`: BusinessCommandServerInput_Catalog,
    val `channel`: BusinessCommandCatalogChannel
) : BusinessCommand { override val `operation`: String = "billingCatalog" }
data class BusinessCommand_promotionFailure(
    val `error`: String?,
    val `status`: Int?
) : BusinessCommand { override val `operation`: String = "promotionFailure" }
data class BusinessCommand_normalizePromotion(
    val `code`: String
) : BusinessCommand { override val `operation`: String = "normalizePromotion" }
data class BusinessCommand_resolvePromotionOfferHint(
    val `available_offers`: List<String>,
    val `requested_offer`: String
) : BusinessCommand { override val `operation`: String = "resolvePromotionOfferHint" }
data class BusinessCommand_inspectPromotionOffer(
    val `offer`: BusinessCommandServerInput_PromotionOffer
) : BusinessCommand { override val `operation`: String = "inspectPromotionOffer" }
data class BusinessCommand_validatePromotionOffer(
    val `catalog`: BusinessCommandPromotionCatalog?,
    val `offer`: BusinessCommandServerInput_PromotionOffer,
    val `requested_offer`: String?
) : BusinessCommand { override val `operation`: String = "validatePromotionOffer" }
