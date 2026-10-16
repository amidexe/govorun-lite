package com.govorun.lite.account

/** An unresolved operation and the preceding subscription are different facts. */
val BillingUiState.subscriptionDetailsAvailable: Boolean
    get() = !statusUnavailable && !checkoutPending

val BillingUiState.purchaseAvailable: Boolean
    get() = !working && subscriptionDetailsAvailable

val BillingUiState.subscriptionInformationUpdating: Boolean
    get() = checkoutChecking || (loading && statusUnavailable)
