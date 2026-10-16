// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingPlanChange(
    val `mode`: String,
    val `productId`: String = "",
    val `targetOfferId`: String = "",
    val `effectiveAt`: String = "",
    val `carryoverLimit`: Int = 0,
    val `clientCheckout`: ConsumerCheckoutPresentation? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("carryoverLimit", `carryoverLimit`)
        .put("clientCheckout", `clientCheckout`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("effectiveAt", `effectiveAt`)
        .put("mode", `mode`)
        .put("productId", `productId`)
        .put("targetOfferId", `targetOfferId`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingPlanChange = ConsumerBillingPlanChange(
            `carryoverLimit` = value.getInt("carryoverLimit"),
            `clientCheckout` = if (value.isNull("clientCheckout")) null else ConsumerCheckoutPresentation.fromJson(value.getJSONObject("clientCheckout")),
            `effectiveAt` = value.getString("effectiveAt"),
            `mode` = value.getString("mode"),
            `productId` = value.getString("productId"),
            `targetOfferId` = value.getString("targetOfferId")
        )
    }
}
