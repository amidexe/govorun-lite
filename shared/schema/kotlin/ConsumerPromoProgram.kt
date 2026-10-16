// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoProgram(
    val `campaignId`: String = "",
    val `programVersion`: String = "",
    val `state`: String = "disabled",
    val `enrollmentOpen`: Boolean = false,
    val `activationAvailable`: Boolean = false,
    val `participated`: Boolean = false,
    val `accessMode`: String = "common_end",
    val `accessDays`: Int = 0,
    val `accessEndsAt`: String? = null,
    val `endsAt`: String? = null,
    val `dailyLimit`: Int = 0,
    val `periodLimit`: Int = 0,
    val `periodDays`: Int = 0,
    val `message`: String = "",
    val `visible`: Boolean = false,
    val `canActivate`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessDays", `accessDays`)
        .put("accessEndsAt", `accessEndsAt`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("accessMode", `accessMode`)
        .put("activationAvailable", `activationAvailable`)
        .put("campaignId", `campaignId`)
        .put("canActivate", `canActivate`)
        .put("dailyLimit", `dailyLimit`)
        .put("endsAt", `endsAt`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("enrollmentOpen", `enrollmentOpen`)
        .put("message", `message`)
        .put("participated", `participated`)
        .put("periodDays", `periodDays`)
        .put("periodLimit", `periodLimit`)
        .put("programVersion", `programVersion`)
        .put("state", `state`)
        .put("visible", `visible`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoProgram = ConsumerPromoProgram(
            `accessDays` = value.getInt("accessDays"),
            `accessEndsAt` = if (value.isNull("accessEndsAt")) null else value.getString("accessEndsAt"),
            `accessMode` = value.getString("accessMode"),
            `activationAvailable` = value.getBoolean("activationAvailable"),
            `campaignId` = value.getString("campaignId"),
            `canActivate` = value.getBoolean("canActivate"),
            `dailyLimit` = value.getInt("dailyLimit"),
            `endsAt` = if (value.isNull("endsAt")) null else value.getString("endsAt"),
            `enrollmentOpen` = value.getBoolean("enrollmentOpen"),
            `message` = value.getString("message"),
            `participated` = value.getBoolean("participated"),
            `periodDays` = value.getInt("periodDays"),
            `periodLimit` = value.getInt("periodLimit"),
            `programVersion` = value.getString("programVersion"),
            `state` = value.getString("state"),
            `visible` = value.getBoolean("visible")
        )
    }
}
