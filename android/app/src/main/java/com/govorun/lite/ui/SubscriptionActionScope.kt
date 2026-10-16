package com.govorun.lite.ui

import androidx.activity.ComponentActivity
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.account.BillingSessionScope
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.SessionBoundary
import kotlinx.coroutines.launch

internal fun ComponentActivity.launchSubscription(
    block: suspend SessionBoundary<BillingSessionScope>.Lease.() -> Unit,
) {
    val lease = GovorunAccountRepository.billingBoundary.capture()
    lifecycleScope.launch {
        lease.ensureCurrent()
        lease.block()
    }
}
