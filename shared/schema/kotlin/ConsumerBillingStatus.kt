// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingStatus(
    val `subscription`: ConsumerBillingSubscription = ConsumerBillingSubscription(`billingSubjectId` = "", `changeEffectiveAt` = "", `currentPeriodEndsAt` = "", `nextCharge` = null, `offerId` = "", `paymentMethod` = null, `pendingOfferId` = "", `pendingPlanCode` = "", `planCode` = "", `provider` = "", `refundRequest` = null, `renewalIssue` = null, `renewsAutomatically` = false, `serviceAccessState` = "", `state` = "none", `suspendedUntil` = "", `suspensionReason` = ""),
    val `starterGrant`: ConsumerStarterGrant = ConsumerStarterGrant(`activationAvailable` = false, `balanceKind` = "", `creditLimit` = 0, `eligible` = false, `expires` = false, `remainingToIssue` = 0, `state` = "disabled", `trialOffer` = null)
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("starterGrant", `starterGrant`.toJson())
        .put("subscription", `subscription`.toJson())
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingStatus = ConsumerBillingStatus(
            `starterGrant` = ConsumerStarterGrant.fromJson(value.getJSONObject("starterGrant")),
            `subscription` = ConsumerBillingSubscription.fromJson(value.getJSONObject("subscription"))
        )
    }
}
