// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerStyleAccess(
    val `type`: String = "none",
    val `state`: String = "none",
    val `startsAt`: String = "",
    val `endsAt`: String = "",
    val `dailyResetsAt`: String = "",
    val `periodResetsAt`: String = "",
    val `dailyLimit`: Int = 0,
    val `totalLimit`: Int = 0,
    val `issuedToday`: Int = 0,
    val `issuedTotal`: Int = 0,
    val `remainingToday`: Int = 0,
    val `remainingTotal`: Int = 0,
    val `quotaUnit`: String? = null,
    val `charactersPerCredit`: Int = 0,
    val `maximumCreditsPerRequest`: Int = 0,
    val `stylizationProtocol`: String? = null,
    val `planCode`: String = "",
    val `balanceKind`: String = "",
    val `usageBalance`: ConsumerStyleUsageBalance? = null,
    val `reserveReturnProtocol`: String? = null,
    val `nextPeriodAt`: String = "",
    val `periodResetCondition`: String = "",
    val `accessEndsAt`: String = "",
    val `issuanceCountersObserved`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessEndsAt", `accessEndsAt`)
        .put("balanceKind", `balanceKind`)
        .put("charactersPerCredit", `charactersPerCredit`)
        .put("dailyLimit", `dailyLimit`)
        .put("dailyResetsAt", `dailyResetsAt`)
        .put("endsAt", `endsAt`)
        .put("issuanceCountersObserved", `issuanceCountersObserved`)
        .put("issuedToday", `issuedToday`)
        .put("issuedTotal", `issuedTotal`)
        .put("maximumCreditsPerRequest", `maximumCreditsPerRequest`)
        .put("nextPeriodAt", `nextPeriodAt`)
        .put("periodResetCondition", `periodResetCondition`)
        .put("periodResetsAt", `periodResetsAt`)
        .put("planCode", `planCode`)
        .put("quotaUnit", `quotaUnit`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("remainingToday", `remainingToday`)
        .put("remainingTotal", `remainingTotal`)
        .put("reserveReturnProtocol", `reserveReturnProtocol`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("startsAt", `startsAt`)
        .put("state", `state`)
        .put("stylizationProtocol", `stylizationProtocol`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("totalLimit", `totalLimit`)
        .put("type", `type`)
        .put("usageBalance", `usageBalance`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerStyleAccess = ConsumerStyleAccess(
            `accessEndsAt` = value.getString("accessEndsAt"),
            `balanceKind` = value.getString("balanceKind"),
            `charactersPerCredit` = value.getInt("charactersPerCredit"),
            `dailyLimit` = value.getInt("dailyLimit"),
            `dailyResetsAt` = value.getString("dailyResetsAt"),
            `endsAt` = value.getString("endsAt"),
            `issuanceCountersObserved` = value.getBoolean("issuanceCountersObserved"),
            `issuedToday` = value.getInt("issuedToday"),
            `issuedTotal` = value.getInt("issuedTotal"),
            `maximumCreditsPerRequest` = value.getInt("maximumCreditsPerRequest"),
            `nextPeriodAt` = value.getString("nextPeriodAt"),
            `periodResetCondition` = value.getString("periodResetCondition"),
            `periodResetsAt` = value.getString("periodResetsAt"),
            `planCode` = value.getString("planCode"),
            `quotaUnit` = if (value.isNull("quotaUnit")) null else value.getString("quotaUnit"),
            `remainingToday` = value.getInt("remainingToday"),
            `remainingTotal` = value.getInt("remainingTotal"),
            `reserveReturnProtocol` = if (value.isNull("reserveReturnProtocol")) null else value.getString("reserveReturnProtocol"),
            `startsAt` = value.getString("startsAt"),
            `state` = value.getString("state"),
            `stylizationProtocol` = if (value.isNull("stylizationProtocol")) null else value.getString("stylizationProtocol"),
            `totalLimit` = value.getInt("totalLimit"),
            `type` = value.getString("type"),
            `usageBalance` = if (value.isNull("usageBalance")) null else ConsumerStyleUsageBalance.fromJson(value.getJSONObject("usageBalance"))
        )
    }
}
