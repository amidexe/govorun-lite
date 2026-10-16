package com.govorun.lite.ui

import android.content.Intent
import androidx.activity.ComponentActivity
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.BillingCatalog
import com.govorun.lite.account.BillingRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal fun cancelSubscriptionRenewal(demoMode: Boolean, scope: CoroutineScope) {
    if (demoMode) {
        BillingRepository.simulateCancellation()
    } else {
        scope.launch { runCatching { BillingRepository.cancelRenewal() } }
    }
}

internal fun openSubscriptionSignIn(activity: ComponentActivity) {
    activity.startActivity(
        Intent(activity, AccountActivity::class.java)
            .putExtra(AccountActivity.EXTRA_RETURN_TO_SUBSCRIPTION, true),
    )
}

internal fun openCoreSetup(activity: ComponentActivity) {
    activity.startActivity(
        Intent(activity, OnboardingActivity::class.java)
            .putExtra(OnboardingActivity.EXTRA_RECOVERY, true),
    )
}

internal fun billingClientSupportsCatalog(catalog: BillingCatalog, demoMode: Boolean): Boolean =
    demoMode || catalog.tBankPreviewEnabled

internal fun acceptsTBankReturn(intent: Intent): Boolean = acceptsTBankReturn(
    action = intent.action,
    scheme = intent.data?.scheme,
    host = intent.data?.host,
    path = intent.data?.path,
    returnScheme = BuildConfig.TBANK_RETURN_SCHEME,
)

internal fun acceptsTBankReturn(
    action: String?,
    scheme: String?,
    host: String?,
    path: String?,
    returnScheme: String = "govorun",
): Boolean {
    if (action != Intent.ACTION_VIEW) return false
    if (scheme == returnScheme) return host == "billing" && path == "/tbank-return"
    if (returnScheme != "govorun") return false
    return scheme == "https" && host == "govorun.club" && path in setOf(
        "/payment/success",
        "/payment/failure",
        "/payment/recovery",
    )
}

internal fun cancelScheduledSubscriptionChange(scope: CoroutineScope) {
    scope.launch { runCatching { BillingRepository.cancelScheduledPlanChange() } }
}
