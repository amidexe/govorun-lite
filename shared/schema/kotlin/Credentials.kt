// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class Credentials(
    val `accessToken`: String,
    val `refreshToken`: String,
    val `deviceId`: String = "",
    val `deviceLimit`: Int = 0,
    val `displayName`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessToken", `accessToken`)
        .put("deviceId", `deviceId`)
        .put("deviceLimit", `deviceLimit`)
        .put("displayName", `displayName`)
        .put("refreshToken", `refreshToken`)
    companion object {
        fun fromJson(value: org.json.JSONObject): Credentials = Credentials(
            `accessToken` = value.getString("accessToken"),
            `deviceId` = value.getString("deviceId"),
            `deviceLimit` = value.getInt("deviceLimit"),
            `displayName` = value.getString("displayName"),
            `refreshToken` = value.getString("refreshToken")
        )
    }
}
