// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoRedemption(
    val `redemptionId`: String,
    val `redeemedAt`: String,
    val `offer`: ConsumerPromoOffer
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("offer", `offer`.toJson())
        .put("redeemedAt", `redeemedAt`)
        .put("redemptionId", `redemptionId`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoRedemption = ConsumerPromoRedemption(
            `offer` = ConsumerPromoOffer.fromJson(value.getJSONObject("offer")),
            `redeemedAt` = value.getString("redeemedAt"),
            `redemptionId` = value.getString("redemptionId")
        )
    }
}
