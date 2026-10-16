package com.govorun.lite.ui

import com.govorun.lite.account.SubscriptionAccess
import com.govorun.lite.account.SubscriptionBilling
import com.govorun.lite.account.SubscriptionFailure

import android.content.Context
import com.govorun.lite.account.BillingRepository
import com.govorun.lite.account.BillingActions
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.model.billing.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.StateFlow

internal class AndroidSubscriptionBilling(private val context: Context) : SubscriptionBilling {
    override val access: SubscriptionAccess
        get() {
            val account = GovorunAccountRepository.state.value
            return SubscriptionAccess(
                account.signedIn, account.hasEntitlement,
                account.freeAccessAvailable,
                account.promoProgram,
            )
        }
    val managementActions: StateFlow<BillingActions> get() = BillingRepository.actions
    override val billing get() = BillingRepository.state.value
    override suspend fun load() = BillingRepository.load()
    override val promotions get() = BillingRepository.promotions.value
    override val window get() = BillingRepository.window.value
    override suspend fun checkout(command: com.govorun.lite.account.SubscriptionCommand) = BillingRepository.subscription(command)
    override fun interaction(action: String, accepted: Boolean, handoffPending: Boolean) =
        com.govorun.lite.account.subscriptionWindowDecision(action, access, accepted, handoffPending)
    override fun purchaseFailed() = BillingRepository.purchaseFailed()
    override suspend fun dismissCheckoutForm() = BillingRepository.dismissCheckoutForm()

}
