package com.govorun.lite.account

import com.govorun.lite.model.billing.*

data class SubscriptionAccess(
    val signedIn: Boolean,
    val hasEntitlement: Boolean,
    val starterEligible: Boolean,
    val promoProgram: PromoProgram,
)

enum class PaymentLaunch { OPENED, FAILED, TEST_BROWSER_MISSING }

/** Transport adapters provide safe user copy. */
class SubscriptionFailure(val notice: String, val discardCheckoutConfirmation: Boolean = false) : Exception(notice)

enum class PromotionAccessKind { DONE, ACCEPT_TERMS, REFRESH }
data class PromotionAccess(
    val kind: PromotionAccessKind,
    val termsVersion: Int?,
    val message: String,
    val deferredMessage: String,
)

interface SubscriptionBilling {
    val access: SubscriptionAccess
    val billing: BillingUiState
    suspend fun load()
    val promotions: PromotionWindow
    val window: SubscriptionWindow
    suspend fun checkout(command: SubscriptionCommand): SubscriptionEffect?
    fun interaction(action: String, accepted: Boolean = false, handoffPending: Boolean = false): SubscriptionWindowDecision
    fun purchaseFailed()
    suspend fun dismissCheckoutForm()
}

/** Only this edge knows dialogs, browser invocation and local continuation storage. */
interface SubscriptionEffects {
    fun signIn()
    fun beginStarterHandoff()
    fun starterHandoffPending(): Boolean
    fun clearStarterHandoff()
    fun dismissStarterGuidance()
    suspend fun activateStarterWithConsent(program: PromoProgram): Boolean
    suspend fun requestStyleConsent(): Boolean
    fun openPayment(url: String): PaymentLaunch
    fun refreshCheckout()
}

data class SubscriptionPresentation(
    val starterWorking: Boolean = false,
    val consentPromptOpen: Boolean = false,
    val promoMessage: String? = null,
)

/** Values returned by the common owner, interpreted only as UI instructions. */
data class SubscriptionWindowDecision(val kind: String, val message: String?)
