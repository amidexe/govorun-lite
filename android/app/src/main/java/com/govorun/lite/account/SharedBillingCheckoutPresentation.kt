package com.govorun.lite.account

import com.govorun.lite.model.billing.BillingCheckoutPresentation
import com.govorun.lite.model.billing.CheckoutMessageUpdate
import org.json.JSONObject

internal fun JSONObject.toBillingCheckoutPresentation() =
    com.govorun.core.generated.ConsumerBillingCheckoutPresentation.fromJson(consumerView("billingCheckoutPresentation"))

internal fun BillingUiState.projectCheckoutMessage(
    message: String? = checkoutMessage,
    pending: Boolean = checkoutPending,
    scheduled: Boolean = checkoutScheduled,
): String? = when {
    pending -> message
    scheduled -> checkoutPresentation.scheduled.let { if (it.replace) it.message else message }
    else -> checkoutPresentation.settled.let { if (it.replace) it.message else message }
}
