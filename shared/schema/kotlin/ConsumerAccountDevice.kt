// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerAccountDevice(
    val `id`: String,
    val `platform`: String,
    val `displayName`: String,
    val `activatedAt`: String,
    val `lastSeenAt`: String,
    val `current`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("activatedAt", `activatedAt`)
        .put("current", `current`)
        .put("displayName", `displayName`)
        .put("id", `id`)
        .put("lastSeenAt", `lastSeenAt`)
        .put("platform", `platform`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerAccountDevice = ConsumerAccountDevice(
            `activatedAt` = value.getString("activatedAt"),
            `current` = value.getBoolean("current"),
            `displayName` = value.getString("displayName"),
            `id` = value.getString("id"),
            `lastSeenAt` = value.getString("lastSeenAt"),
            `platform` = value.getString("platform")
        )
    }
}
