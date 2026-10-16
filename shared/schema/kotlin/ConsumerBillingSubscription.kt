// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingSubscription(
    val `state`: String = "none",
    val `planCode`: String = "",
    val `offerId`: String = "",
    val `provider`: String = "",
    val `currentPeriodEndsAt`: String = "",
    val `renewsAutomatically`: Boolean = false,
    val `pendingPlanCode`: String = "",
    val `pendingOfferId`: String = "",
    val `changeEffectiveAt`: String = "",
    val `billingSubjectId`: String = "",
    val `serviceAccessState`: String = "",
    val `suspensionReason`: String = "",
    val `suspendedUntil`: String = "",
    val `refundRequest`: ConsumerBillingRefundRequest? = null,
    val `renewalIssue`: ConsumerBillingRenewalIssue? = null,
    val `nextCharge`: ConsumerBillingNextCharge? = null,
    val `paymentMethod`: ConsumerBillingPaymentMethod? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("billingSubjectId", `billingSubjectId`)
        .put("changeEffectiveAt", `changeEffectiveAt`)
        .put("currentPeriodEndsAt", `currentPeriodEndsAt`)
        .put("nextCharge", `nextCharge`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("offerId", `offerId`)
        .put("paymentMethod", `paymentMethod`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("pendingOfferId", `pendingOfferId`)
        .put("pendingPlanCode", `pendingPlanCode`)
        .put("planCode", `planCode`)
        .put("provider", `provider`)
        .put("refundRequest", `refundRequest`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("renewalIssue", `renewalIssue`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("renewsAutomatically", `renewsAutomatically`)
        .put("serviceAccessState", `serviceAccessState`)
        .put("state", `state`)
        .put("suspendedUntil", `suspendedUntil`)
        .put("suspensionReason", `suspensionReason`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingSubscription = ConsumerBillingSubscription(
            `billingSubjectId` = value.getString("billingSubjectId"),
            `changeEffectiveAt` = value.getString("changeEffectiveAt"),
            `currentPeriodEndsAt` = value.getString("currentPeriodEndsAt"),
            `nextCharge` = if (value.isNull("nextCharge")) null else ConsumerBillingNextCharge.fromJson(value.getJSONObject("nextCharge")),
            `offerId` = value.getString("offerId"),
            `paymentMethod` = if (value.isNull("paymentMethod")) null else ConsumerBillingPaymentMethod.fromJson(value.getJSONObject("paymentMethod")),
            `pendingOfferId` = value.getString("pendingOfferId"),
            `pendingPlanCode` = value.getString("pendingPlanCode"),
            `planCode` = value.getString("planCode"),
            `provider` = value.getString("provider"),
            `refundRequest` = if (value.isNull("refundRequest")) null else ConsumerBillingRefundRequest.fromJson(value.getJSONObject("refundRequest")),
            `renewalIssue` = if (value.isNull("renewalIssue")) null else ConsumerBillingRenewalIssue.fromJson(value.getJSONObject("renewalIssue")),
            `renewsAutomatically` = value.getBoolean("renewsAutomatically"),
            `serviceAccessState` = value.getString("serviceAccessState"),
            `state` = value.getString("state"),
            `suspendedUntil` = value.getString("suspendedUntil"),
            `suspensionReason` = value.getString("suspensionReason")
        )
    }
}
