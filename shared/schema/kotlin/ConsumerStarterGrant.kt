// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerStarterGrant(
    val `state`: String = "disabled",
    val `creditLimit`: Int = 0,
    val `remainingToIssue`: Int = 0,
    val `activationAvailable`: Boolean = false,
    val `expires`: Boolean = false,
    val `balanceKind`: String = "",
    val `trialOffer`: ConsumerBillingTrialOffer? = null,
    val `eligible`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("activationAvailable", `activationAvailable`)
        .put("balanceKind", `balanceKind`)
        .put("creditLimit", `creditLimit`)
        .put("eligible", `eligible`)
        .put("expires", `expires`)
        .put("remainingToIssue", `remainingToIssue`)
        .put("state", `state`)
        .put("trialOffer", `trialOffer`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerStarterGrant = ConsumerStarterGrant(
            `activationAvailable` = value.getBoolean("activationAvailable"),
            `balanceKind` = value.getString("balanceKind"),
            `creditLimit` = value.getInt("creditLimit"),
            `eligible` = value.getBoolean("eligible"),
            `expires` = value.getBoolean("expires"),
            `remainingToIssue` = value.getInt("remainingToIssue"),
            `state` = value.getString("state"),
            `trialOffer` = if (value.isNull("trialOffer")) null else ConsumerBillingTrialOffer.fromJson(value.getJSONObject("trialOffer"))
        )
    }
}
