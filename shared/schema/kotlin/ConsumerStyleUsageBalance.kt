// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerStyleUsageBalance(
    val `scopeId`: String,
    val `revision`: Long,
    val `day`: String,
    val `serverTime`: String,
    val `consumedToday`: Int,
    val `consumedTotal`: Int,
    val `remainingToday`: Int,
    val `remainingTotal`: Int,
    val `deviceDays`: Map<String, Int> = mapOf(),
    val `settlementScopeIds`: List<String> = listOf(),
    val `dailySettlementScopeIds`: List<String> = listOf(),
    val `deviceRequests`: Map<String, ConsumerStyleRequestCount> = mapOf(),
    val `version`: Int = 1
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("consumedToday", `consumedToday`)
        .put("consumedTotal", `consumedTotal`)
        .put("dailySettlementScopeIds", org.json.JSONArray(`dailySettlementScopeIds`.map { item -> item }))
        .put("day", `day`)
        .put("deviceDays", org.json.JSONObject(`deviceDays`.mapValues { (_, item) -> item }))
        .put("deviceRequests", org.json.JSONObject(`deviceRequests`.mapValues { (_, item) -> item.toJson() }))
        .put("remainingToday", `remainingToday`)
        .put("remainingTotal", `remainingTotal`)
        .put("revision", `revision`)
        .put("scopeId", `scopeId`)
        .put("serverTime", `serverTime`)
        .put("settlementScopeIds", org.json.JSONArray(`settlementScopeIds`.map { item -> item }))
        .put("version", `version`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerStyleUsageBalance = ConsumerStyleUsageBalance(
            `consumedToday` = value.getInt("consumedToday"),
            `consumedTotal` = value.getInt("consumedTotal"),
            `dailySettlementScopeIds` = value.getJSONArray("dailySettlementScopeIds").let { values -> List(values.length()) { index -> values.getString(index) } },
            `day` = value.getString("day"),
            `deviceDays` = value.getJSONObject("deviceDays").let { values -> values.keys().asSequence().associateWith { key -> values.getInt(key) } },
            `deviceRequests` = value.getJSONObject("deviceRequests").let { values -> values.keys().asSequence().associateWith { key -> ConsumerStyleRequestCount.fromJson(values.getJSONObject(key)) } },
            `remainingToday` = value.getInt("remainingToday"),
            `remainingTotal` = value.getInt("remainingTotal"),
            `revision` = value.getLong("revision"),
            `scopeId` = value.getString("scopeId"),
            `serverTime` = value.getString("serverTime"),
            `settlementScopeIds` = value.getJSONArray("settlementScopeIds").let { values -> List(values.length()) { index -> values.getString(index) } },
            `version` = value.getInt("version")
        )
    }
}
