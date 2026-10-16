// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingTrialOffer(
    val `state`: String = "disabled",
    val `eligible`: Boolean = false,
    val `durationDays`: Int = 0,
    val `requiresVerifiedPaymentMethod`: Boolean = true,
    val `checkoutOpenGrantsAccess`: Boolean = false,
    val `requiresLocalReserveEmpty`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("checkoutOpenGrantsAccess", `checkoutOpenGrantsAccess`)
        .put("durationDays", `durationDays`)
        .put("eligible", `eligible`)
        .put("requiresLocalReserveEmpty", `requiresLocalReserveEmpty`)
        .put("requiresVerifiedPaymentMethod", `requiresVerifiedPaymentMethod`)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingTrialOffer = ConsumerBillingTrialOffer(
            `checkoutOpenGrantsAccess` = value.getBoolean("checkoutOpenGrantsAccess"),
            `durationDays` = value.getInt("durationDays"),
            `eligible` = value.getBoolean("eligible"),
            `requiresLocalReserveEmpty` = value.getBoolean("requiresLocalReserveEmpty"),
            `requiresVerifiedPaymentMethod` = value.getBoolean("requiresVerifiedPaymentMethod"),
            `state` = value.getString("state")
        )
    }
}
