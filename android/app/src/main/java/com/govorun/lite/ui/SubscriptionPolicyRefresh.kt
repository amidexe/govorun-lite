package com.govorun.lite.ui

import androidx.activity.ComponentActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.govorun.lite.account.BillingRepository
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.billingSessionScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch

/** Refresh only while visible. Foreground entry still refreshes immediately. */
internal fun ComponentActivity.observeSubscriptionPolicy(
    onSessionChanged: () -> Unit,
) {
    var previousScope = billingSessionScope(GovorunAccountRepository.state.value)
    lifecycleScope.launch {
        repeatOnLifecycle(Lifecycle.State.STARTED) {
            launch {
                GovorunAccountRepository.state.map(::billingSessionScope)
                    .distinctUntilChanged().collectLatest { scope ->
                        if (scope != previousScope) onSessionChanged()
                        previousScope = scope
                        BillingRepository.load()
                    }
            }
            launch {
                while (true) {
                    delay(BillingRepository.state.value.catalog.refreshAfterSeconds.coerceIn(60, 3_600) * 1_000L)
                    BillingRepository.load()
                }
            }
        }
    }
}
