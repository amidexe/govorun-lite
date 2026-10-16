package com.govorun.lite.account

import com.govorun.lite.account.SessionBoundary
import kotlinx.coroutines.flow.StateFlow

/** Application-wide composition; the store owns billing state, never the account identity. */
internal object BillingRepository : BillingService by BillingStore(AndroidBillingEnvironment()) {
    val actions: StateFlow<BillingActions> get() = GovorunAccountRepository.billingActions
}

internal interface BillingEnvironment {
    val boundary: SessionBoundary<BillingSessionScope>
    suspend fun accountCommand(command: org.json.JSONObject): org.json.JSONObject
    fun accountProjection(): org.json.JSONObject
}

private class AndroidBillingEnvironment : BillingEnvironment {
    override val boundary get() = GovorunAccountRepository.billingBoundary
    override suspend fun accountCommand(command: org.json.JSONObject) = GovorunAccountRepository.sharedAccountCommand(command)
    override fun accountProjection() = GovorunAccountRepository.sharedAccountProjection()
}

internal interface BillingService {
    val state: StateFlow<BillingUiState>
    suspend fun load()
    val promotions: StateFlow<PromotionWindow>
    val window: StateFlow<SubscriptionWindow>
    suspend fun subscription(command: SubscriptionCommand): SubscriptionEffect?
    fun acceptAccountProjection(value: org.json.JSONObject)
    suspend fun cancelScheduledPlanChange()
    suspend fun cancelRenewal()
    suspend fun removePaymentMethod()
    suspend fun requestRefund(reasonCode: String)
    fun simulatePurchase(planCode: String)
    fun simulateCancellation()
    fun purchaseFailed()
    suspend fun dismissCheckoutForm()
    fun clear()
}
