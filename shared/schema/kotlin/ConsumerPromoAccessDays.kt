// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoAccessDays(
    val `days`: Int,
    val `planCode`: String,
    val `planName`: String,
    val `startsAt`: String,
    val `endsAt`: String
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("days", `days`)
        .put("endsAt", `endsAt`)
        .put("planCode", `planCode`)
        .put("planName", `planName`)
        .put("startsAt", `startsAt`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoAccessDays = ConsumerPromoAccessDays(
            `days` = value.getInt("days"),
            `endsAt` = value.getString("endsAt"),
            `planCode` = value.getString("planCode"),
            `planName` = value.getString("planName"),
            `startsAt` = value.getString("startsAt")
        )
    }
}
