// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerProductAccess(
    val `product`: String,
    val `scopeId`: String,
    val `day`: String,
    val `dailyLimit`: Int,
    val `totalLimit`: Int,
    val `consumedToday`: Int,
    val `consumedTotal`: Int,
    val `remainingToday`: Int,
    val `remainingTotal`: Int,
    val `deviceDays`: Map<String, Int> = mapOf()
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("consumedToday", `consumedToday`)
        .put("consumedTotal", `consumedTotal`)
        .put("dailyLimit", `dailyLimit`)
        .put("day", `day`)
        .put("deviceDays", org.json.JSONObject(`deviceDays`.mapValues { (_, item) -> item }))
        .put("product", `product`)
        .put("remainingToday", `remainingToday`)
        .put("remainingTotal", `remainingTotal`)
        .put("scopeId", `scopeId`)
        .put("totalLimit", `totalLimit`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerProductAccess = ConsumerProductAccess(
            `consumedToday` = value.getInt("consumedToday"),
            `consumedTotal` = value.getInt("consumedTotal"),
            `dailyLimit` = value.getInt("dailyLimit"),
            `day` = value.getString("day"),
            `deviceDays` = value.getJSONObject("deviceDays").let { values -> values.keys().asSequence().associateWith { key -> values.getInt(key) } },
            `product` = value.getString("product"),
            `remainingToday` = value.getInt("remainingToday"),
            `remainingTotal` = value.getInt("remainingTotal"),
            `scopeId` = value.getString("scopeId"),
            `totalLimit` = value.getInt("totalLimit")
        )
    }
}
