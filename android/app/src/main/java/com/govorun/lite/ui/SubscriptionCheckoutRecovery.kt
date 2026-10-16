package com.govorun.lite.ui

import androidx.activity.ComponentActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.govorun.lite.account.BillingRepository
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.SubscriptionCommand
import com.govorun.lite.account.SubscriptionEffect
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.map

/** The Activity schedules foreground checks; the common owner queues and reconciles them. */
internal class SubscriptionCheckoutRecovery(private val activity: ComponentActivity) {
    fun observe() {
        activity.lifecycleScope.launch {
            activity.repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch {
                    GovorunAccountRepository.state.map { it.networkAvailable }.distinctUntilChanged().drop(1).collect { connected ->
                        if (connected && !BillingRepository.window.value.busy) request(SubscriptionCommand.Refresh)
                    }
                }
                while (isActive) {
                    request(SubscriptionCommand.Recover)
                    delay(30_000)
                }
            }
        }
    }

    fun refresh(full: Boolean = false) {
        activity.lifecycleScope.launch { request(if (full) SubscriptionCommand.RefreshByUser else SubscriptionCommand.RecoverByUser) }
    }
    fun resumeCheckout() { activity.lifecycleScope.launch { request(SubscriptionCommand.Resume) } }
    fun restartBinding() { activity.lifecycleScope.launch { request(SubscriptionCommand.RestartBinding) } }

    private suspend fun request(command: SubscriptionCommand) {
        val lease = GovorunAccountRepository.billingBoundary.capture()
        try {
            when (val effect = lease.request { BillingRepository.subscription(command) }) {
                null -> Unit
                is SubscriptionEffect.PromotionAccess -> Unit
                is SubscriptionEffect.OpenUrl -> lease.publish {
                    if (!activity.isFinishing && !activity.isDestroyed) {
                        launchBillingPayment(activity, effect.url).onFailure { BillingRepository.purchaseFailed() }
                    }
                }
            }
        } catch (error: CancellationException) { throw error }
        catch (_: Exception) { /* The common projection retains uncertainty and presents the failure. */ }
    }
}
