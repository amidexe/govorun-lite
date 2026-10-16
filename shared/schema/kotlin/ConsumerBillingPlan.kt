// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingPlan(
    val `code`: String,
    val `name`: String,
    val `priceKopeks`: Int,
    val `dailyLimit`: Int,
    val `periodLimit`: Int,
    val `periodDays`: Int,
    val `recommended`: Boolean = false,
    val `ruStoreProductId`: String = "",
    val `offerId`: String = code,
    val `billingPeriodLabel`: String = "$periodDays дней",
    val `clientMultiplierCopy`: String? = null,
    val `clientLimitCopy`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("billingPeriodLabel", `billingPeriodLabel`)
        .put("clientLimitCopy", `clientLimitCopy`)
        .put("clientMultiplierCopy", `clientMultiplierCopy`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("code", `code`)
        .put("dailyLimit", `dailyLimit`)
        .put("name", `name`)
        .put("offerId", `offerId`)
        .put("periodDays", `periodDays`)
        .put("periodLimit", `periodLimit`)
        .put("priceKopeks", `priceKopeks`)
        .put("recommended", `recommended`)
        .put("ruStoreProductId", `ruStoreProductId`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingPlan = ConsumerBillingPlan(
            `billingPeriodLabel` = value.getString("billingPeriodLabel"),
            `clientLimitCopy` = value.getString("clientLimitCopy"),
            `clientMultiplierCopy` = if (value.isNull("clientMultiplierCopy")) null else value.getString("clientMultiplierCopy"),
            `code` = value.getString("code"),
            `dailyLimit` = value.getInt("dailyLimit"),
            `name` = value.getString("name"),
            `offerId` = value.getString("offerId"),
            `periodDays` = value.getInt("periodDays"),
            `periodLimit` = value.getInt("periodLimit"),
            `priceKopeks` = value.getInt("priceKopeks"),
            `recommended` = value.getBoolean("recommended"),
            `ruStoreProductId` = value.getString("ruStoreProductId")
        )
    }
}
