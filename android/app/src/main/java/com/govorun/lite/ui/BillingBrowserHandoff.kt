package com.govorun.lite.ui

import android.app.Activity
import android.net.Uri
import androidx.browser.customtabs.CustomTabsIntent
import com.govorun.lite.BuildConfig

internal fun launchBillingPayment(activity: Activity, url: String): Result<Unit> =
    launchBillingBrowser(BuildConfig.BILLING_BROWSER_PACKAGE) { browserPackage ->
        CustomTabsIntent.Builder().build().apply {
            intent.setPackage(browserPackage)
        }.launchUrl(activity, Uri.parse(url))
    }

/** A fixed test browser must never fall back to the device's direct browser route. */
internal fun launchBillingBrowser(
    configuredPackage: String,
    launch: (String?) -> Unit,
): Result<Unit> = runCatching {
    launch(configuredPackage.takeIf(String::isNotEmpty))
}

internal fun billingLegalDocumentHandler(activity: Activity): ((String) -> Result<Unit>)? =
    billingLegalDocumentHandler(BuildConfig.BILLING_BROWSER_PACKAGE) { url ->
        launchBillingPayment(activity, url)
    }

/** Ordinary links keep LocalUriHandler; the isolated profile uses its fixed browser. */
internal fun billingLegalDocumentHandler(
    configuredPackage: String,
    openFixed: (String) -> Result<Unit>,
): ((String) -> Result<Unit>)? = openFixed.takeIf { configuredPackage.isNotEmpty() }
