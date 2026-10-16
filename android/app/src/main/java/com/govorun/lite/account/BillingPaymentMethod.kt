package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

typealias BillingPaymentMethod = com.govorun.lite.model.billing.BillingPaymentMethod

private fun JSONObject.methodView(fresh: Boolean = true): JSONObject = SharedBusiness.evaluate(
    JSONObject().put("operation", "billingPaymentMethodView").put("method", this).put("statusAvailable", fresh))

internal fun JSONObject.toBillingPaymentMethod() =
    com.govorun.core.generated.ConsumerBillingPaymentMethod.fromJson(consumerView("billingPaymentMethod"))

internal data class BillingPaymentMethodPresentation(val text: String, val needsRefresh: Boolean)

internal fun BillingPaymentMethod.presentation(statusAvailable: Boolean = true): BillingPaymentMethodPresentation {
    val view = JSONObject().put("state", state).put("lastFour", lastFour).put("kind", kind)
        .put("available", available).methodView(statusAvailable)
    return BillingPaymentMethodPresentation(view.getString("text"), view.getBoolean("needsRefresh"))
}
