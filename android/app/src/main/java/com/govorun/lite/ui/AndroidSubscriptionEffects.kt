package com.govorun.lite.ui

import com.govorun.lite.account.SubscriptionEffects
import com.govorun.lite.account.PaymentLaunch

import androidx.activity.ComponentActivity
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.StarterActivationHandoff
import com.govorun.lite.model.billing.*
import com.govorun.lite.stylization.StylizationConsent
import com.govorun.lite.util.HomePreferences
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

internal class AndroidSubscriptionEffects(
    private val activity: ComponentActivity,
    private val recovery: SubscriptionCheckoutRecovery,
) : SubscriptionEffects {
    override fun signIn() = openSubscriptionSignIn(activity)
    override fun beginStarterHandoff() = StarterActivationHandoff.begin(activity)
    override fun starterHandoffPending() = StarterActivationHandoff.pending(activity)
    override fun clearStarterHandoff() = StarterActivationHandoff.clear(activity)
    override fun dismissStarterGuidance() = HomePreferences.dismissStarterGuidance(activity)

    override suspend fun activateStarterWithConsent(program: PromoProgram): Boolean {
        val lease = GovorunAccountRepository.billingBoundary.capture()
        return callback { complete ->
            StylizationConsent.requestEnable(
                activity = activity,
                allowBeforeEntitlement = true,
                acceptAccessAndTerms = { style, terms -> lease.request {
                    GovorunAccountRepository.activateStarterGrant(activity, terms, style, program)
                } },
                onResult = complete,
            )
        }
    }

    override suspend fun requestStyleConsent(): Boolean = callback { complete ->
        StylizationConsent.requestEnable(activity = activity, onResult = complete)
    }

    override fun openPayment(url: String): PaymentLaunch = when {
        launchBillingPayment(activity, url).isSuccess -> PaymentLaunch.OPENED
        BuildConfig.BILLING_BROWSER_PACKAGE.isNotEmpty() -> PaymentLaunch.TEST_BROWSER_MISSING
        else -> PaymentLaunch.FAILED
    }

    override fun refreshCheckout() = recovery.refresh()

    private suspend fun <T> callback(start: ((T) -> Unit) -> Unit): T = suspendCancellableCoroutine { continuation ->
        val delivered = AtomicBoolean(false)
        start { value ->
            if (delivered.compareAndSet(false, true) && continuation.isActive) continuation.resume(value)
        }
    }
}
