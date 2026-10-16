package com.govorun.lite.account

import com.govorun.lite.model.billing.CheckoutPresentation
import org.json.JSONObject

/** The shared journal can contain a bank checkout or a change without payment. */
internal sealed interface CheckoutOutcome {
    data class Bank(val session: TBankCheckoutSession) : CheckoutOutcome
    data class PlanChange(val change: BillingPlanChange) : CheckoutOutcome
}

internal fun JSONObject.toCheckoutPresentation() =
    com.govorun.core.generated.ConsumerCheckoutPresentation.fromJson(consumerView("checkoutPresentation"))
internal fun JSONObject.toSharedPlanChange() =
    com.govorun.core.generated.ConsumerBillingPlanChange.fromJson(consumerView("billingPlanChange"))

internal fun JSONObject.toCheckoutOutcome(): CheckoutOutcome =
    if (getJSONObject("clientCheckout").optString("kind") == "plan_change")
        CheckoutOutcome.PlanChange(toSharedPlanChange())
    else CheckoutOutcome.Bank(toTBankCheckoutSession())
