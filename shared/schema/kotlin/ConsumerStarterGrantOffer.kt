// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerStarterGrantOffer(
    val `clientOfferVisible`: Boolean = false,
    val `state`: String = "disabled",
    val `creditLimit`: Int = 0,
    val `requiresAuthentication`: Boolean = true,
    val `requiresCode`: Boolean = false,
    val `requiresPaymentMethod`: Boolean = false,
    val `expires`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("clientOfferVisible", `clientOfferVisible`)
        .put("creditLimit", `creditLimit`)
        .put("expires", `expires`)
        .put("requiresAuthentication", `requiresAuthentication`)
        .put("requiresCode", `requiresCode`)
        .put("requiresPaymentMethod", `requiresPaymentMethod`)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerStarterGrantOffer = ConsumerStarterGrantOffer(
            `clientOfferVisible` = value.getBoolean("clientOfferVisible"),
            `creditLimit` = value.getInt("creditLimit"),
            `expires` = value.getBoolean("expires"),
            `requiresAuthentication` = value.getBoolean("requiresAuthentication"),
            `requiresCode` = value.getBoolean("requiresCode"),
            `requiresPaymentMethod` = value.getBoolean("requiresPaymentMethod"),
            `state` = value.getString("state")
        )
    }
}
