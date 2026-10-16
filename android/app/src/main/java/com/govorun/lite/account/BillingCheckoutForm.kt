package com.govorun.lite.account

internal const val CHECKOUT_UNAVAILABLE_MESSAGE = "Оплата временно недоступна"

/** Form failures are scoped to the selected offer, separate from authoritative checkout recovery. */
typealias BillingCheckoutFormFailure = com.govorun.lite.model.billing.BillingCheckoutFormFailure

fun BillingUiState.checkoutConfirmationMessage(offerId: String): String? =
    checkoutFormFailure?.takeIf { it.offerId == offerId }?.message

internal fun BillingUiState.checkoutFormFailed(offerId: String, message: String): BillingUiState = copy(
    working = false,
    checkoutFormFailure = BillingCheckoutFormFailure(offerId, message),
)

internal fun BillingUiState.dismissedCheckoutForm(): BillingUiState = copy(checkoutFormFailure = null)
