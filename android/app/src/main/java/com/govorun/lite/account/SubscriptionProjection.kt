package com.govorun.lite.account

import org.json.JSONObject

/** Flattens admitted fields for Compose; no retry, expiry or payment decision lives here. */
internal fun BillingUiState.withSubscriptionProjection(value: JSONObject, window: SubscriptionWindow,
    windowChanged: Boolean): BillingUiState {
    val snapshot = value.getJSONObject("snapshot")
    val catalog = value.getJSONObject("billingCatalog")
    val status = value.getJSONObject("billingStatus")
    val managementMessage = status.get("message").takeUnless { it == JSONObject.NULL } as String?
    val message = window.message ?: managementMessage
    return copy(
        catalog = catalog.optJSONObject("catalog")?.toBillingCatalog() ?: BillingCatalog(),
        loading = catalog.getString("state") == "loading",
        working = if (windowChanged) window.busy else working,
        message = if (windowChanged || message != null) message else this.message,
        messageIsPrimary = window.message != null,
        checkoutMessage = message,
        checkoutFormFailure = null,
        checkoutRefreshFailed = window.failed,
        checkoutScheduled = window.disposition == "scheduled",
        checkoutExpiresAt = window.expiresAt,
        checkoutResumeId = checkoutSessionId.takeIf { window.canResume },
        checkoutResumeUrl = window.checkoutUrl,
        checkoutRestartId = checkoutSessionId.takeIf { window.canCancel },
        statusUnavailable = snapshot.getBoolean("signedIn") && !status.getBoolean("fresh"),
    )
}
