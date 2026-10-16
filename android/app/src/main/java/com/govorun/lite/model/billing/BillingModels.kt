package com.govorun.lite.model.billing

typealias BillingPlan = com.govorun.core.generated.ConsumerBillingPlan

typealias BillingCatalog = com.govorun.core.generated.ConsumerBillingCatalog

typealias StarterGrantOffer = com.govorun.core.generated.ConsumerStarterGrantOffer

typealias BillingTrialOffer = com.govorun.core.generated.ConsumerBillingTrialOffer

typealias BillingStatus = com.govorun.core.generated.ConsumerBillingStatus

typealias BillingSubscription = com.govorun.core.generated.ConsumerBillingSubscription

typealias BillingRenewalIssue = com.govorun.core.generated.ConsumerBillingRenewalIssue

typealias BillingNextCharge = com.govorun.core.generated.ConsumerBillingNextCharge

typealias BillingRefundRequest = com.govorun.core.generated.ConsumerBillingRefundRequest

typealias BillingPlanChange = com.govorun.core.generated.ConsumerBillingPlanChange

typealias TBankCheckoutPreview = com.govorun.core.generated.ConsumerTBankCheckoutPreview

typealias CheckoutPreviewAction = com.govorun.core.generated.ConsumerCheckoutPreviewAction

typealias TBankLegalDocument = com.govorun.core.generated.ConsumerTBankLegalDocument

typealias TBankLegalDocuments = com.govorun.core.generated.ConsumerTBankLegalDocuments

typealias TBankCheckoutSession = com.govorun.core.generated.ConsumerTBankCheckoutSession

/** Decisions supplied by the shared checkout owner; native code only renders them. */
typealias CheckoutPresentation = com.govorun.core.generated.ConsumerCheckoutPresentation

/** Wire directives from the shared billing owner; no subscription policy lives here. */
typealias CheckoutMessageUpdate = com.govorun.core.generated.ConsumerCheckoutMessageUpdate

typealias BillingCheckoutPresentation = com.govorun.core.generated.ConsumerBillingCheckoutPresentation

data class BillingUiState(
    val loading: Boolean = false,
    val working: Boolean = false,
    val catalog: BillingCatalog = BillingCatalog(),
    val subscription: BillingSubscription = BillingSubscription(),
    val starterGrant: StarterGrant = StarterGrant(),
    val message: String? = null,
    val messageIsPrimary: Boolean = false,
    val checkoutMessage: String? = null,
    val checkoutFormFailure: BillingCheckoutFormFailure? = null,
    val checkoutPending: Boolean = false,
    val checkoutPurchasePending: Boolean = false,
    val checkoutSessionId: String? = null,
    val checkoutSessionGeneration: Long? = null,
    val checkoutRestartId: String? = null,
    val checkoutResumeId: String? = null,
    val checkoutResumeUrl: String? = null,
    val checkoutChecking: Boolean = false,
    val checkoutExpiresAt: Long? = null,
    val checkoutRefreshFailed: Boolean = false,
    val statusUnavailable: Boolean = false,
    val checkoutScheduled: Boolean = false,
    val managementStatusPending: Boolean = false,
    val checkoutPresentation: BillingCheckoutPresentation = BillingCheckoutPresentation(),
)
