// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerProfile(
    val `id`: String,
    val `displayName`: String,
    val `currentDeviceId`: String,
    val `deviceLimit`: Int,
    val `onlineTermsVersion`: Int,
    val `styleAccess`: ConsumerStyleAccess,
    val `onlineConsent`: ConsumerOnlineConsent,
    val `onlineServiceState`: String = "unknown",
    val `starterGrant`: ConsumerStarterGrant = ConsumerStarterGrant(`activationAvailable` = false, `balanceKind` = "", `creditLimit` = 0, `eligible` = false, `expires` = false, `remainingToIssue` = 0, `state` = "disabled", `trialOffer` = null),
    val `promoProgram`: ConsumerPromoProgram = ConsumerPromoProgram(`accessDays` = 0, `accessEndsAt` = null, `accessMode` = "common_end", `activationAvailable` = false, `campaignId` = "", `canActivate` = false, `dailyLimit` = 0, `endsAt` = null, `enrollmentOpen` = false, `message` = "", `participated` = false, `periodDays` = 0, `periodLimit` = 0, `programVersion` = "", `state` = "disabled", `visible` = false),
    val `clientFreeAccessAvailable`: Boolean = false,
    val `styleUsageAccountingAvailable`: Boolean = false,
    val `hasSyncedData`: Boolean? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("clientFreeAccessAvailable", `clientFreeAccessAvailable`)
        .put("currentDeviceId", `currentDeviceId`)
        .put("deviceLimit", `deviceLimit`)
        .put("displayName", `displayName`)
        .put("hasSyncedData", `hasSyncedData`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("id", `id`)
        .put("onlineConsent", `onlineConsent`.toJson())
        .put("onlineServiceState", `onlineServiceState`)
        .put("onlineTermsVersion", `onlineTermsVersion`)
        .put("promoProgram", `promoProgram`.toJson())
        .put("starterGrant", `starterGrant`.toJson())
        .put("styleAccess", `styleAccess`.toJson())
        .put("styleUsageAccountingAvailable", `styleUsageAccountingAvailable`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerProfile = ConsumerProfile(
            `clientFreeAccessAvailable` = value.getBoolean("clientFreeAccessAvailable"),
            `currentDeviceId` = value.getString("currentDeviceId"),
            `deviceLimit` = value.getInt("deviceLimit"),
            `displayName` = value.getString("displayName"),
            `hasSyncedData` = if (value.isNull("hasSyncedData")) null else value.getBoolean("hasSyncedData"),
            `id` = value.getString("id"),
            `onlineConsent` = ConsumerOnlineConsent.fromJson(value.getJSONObject("onlineConsent")),
            `onlineServiceState` = value.getString("onlineServiceState"),
            `onlineTermsVersion` = value.getInt("onlineTermsVersion"),
            `promoProgram` = ConsumerPromoProgram.fromJson(value.getJSONObject("promoProgram")),
            `starterGrant` = ConsumerStarterGrant.fromJson(value.getJSONObject("starterGrant")),
            `styleAccess` = ConsumerStyleAccess.fromJson(value.getJSONObject("styleAccess")),
            `styleUsageAccountingAvailable` = value.getBoolean("styleUsageAccountingAvailable")
        )
    }
}
