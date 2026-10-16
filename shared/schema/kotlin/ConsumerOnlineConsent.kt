// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerOnlineConsent(
    val `acceptedVersion`: Int = 0,
    val `acceptedAt`: String = "",
    val `active`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("acceptedAt", `acceptedAt`)
        .put("acceptedVersion", `acceptedVersion`)
        .put("active", `active`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerOnlineConsent = ConsumerOnlineConsent(
            `acceptedAt` = value.getString("acceptedAt"),
            `acceptedVersion` = value.getInt("acceptedVersion"),
            `active` = value.getBoolean("active")
        )
    }
}
