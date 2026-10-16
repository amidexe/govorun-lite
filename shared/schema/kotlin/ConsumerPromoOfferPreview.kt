// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoOfferPreview(
    val `previewId`: String,
    val `previewExpiresAt`: String,
    val `offer`: ConsumerPromoOffer
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("offer", `offer`.toJson())
        .put("previewExpiresAt", `previewExpiresAt`)
        .put("previewId", `previewId`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoOfferPreview = ConsumerPromoOfferPreview(
            `offer` = ConsumerPromoOffer.fromJson(value.getJSONObject("offer")),
            `previewExpiresAt` = value.getString("previewExpiresAt"),
            `previewId` = value.getString("previewId")
        )
    }
}
