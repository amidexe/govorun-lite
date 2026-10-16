// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoOffer(
    val `planCode`: String,
    val `planName`: String,
    val `currency`: String,
    val `billingPeriods`: List<ConsumerPromoBillingPeriod> = listOf(),
    val `followingPriceKopeks`: Int = 0,
    val `followingPeriodDays`: Int = 0,
    val `bonus`: ConsumerPromoBonus? = null,
    val `confirmationTitle`: String,
    val `confirmationBody`: String,
    val `createsSubscription`: Boolean = false,
    val `checkoutRequired`: Boolean = false,
    val `grantsAccessOnConfirmation`: Boolean = false,
    val `catalogVersion`: String = "test",
    val `offerId`: String = "$planCode.monthly",
    val `accessDays`: ConsumerPromoAccessDays? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessDays", `accessDays`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("billingPeriods", org.json.JSONArray(`billingPeriods`.map { item -> item.toJson() }))
        .put("bonus", `bonus`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("catalogVersion", `catalogVersion`)
        .put("checkoutRequired", `checkoutRequired`)
        .put("confirmationBody", `confirmationBody`)
        .put("confirmationTitle", `confirmationTitle`)
        .put("createsSubscription", `createsSubscription`)
        .put("currency", `currency`)
        .put("followingPeriodDays", `followingPeriodDays`)
        .put("followingPriceKopeks", `followingPriceKopeks`)
        .put("grantsAccessOnConfirmation", `grantsAccessOnConfirmation`)
        .put("offerId", `offerId`)
        .put("planCode", `planCode`)
        .put("planName", `planName`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoOffer = ConsumerPromoOffer(
            `accessDays` = if (value.isNull("accessDays")) null else ConsumerPromoAccessDays.fromJson(value.getJSONObject("accessDays")),
            `billingPeriods` = value.getJSONArray("billingPeriods").let { values -> List(values.length()) { index -> ConsumerPromoBillingPeriod.fromJson(values.getJSONObject(index)) } },
            `bonus` = if (value.isNull("bonus")) null else ConsumerPromoBonus.fromJson(value.getJSONObject("bonus")),
            `catalogVersion` = value.getString("catalogVersion"),
            `checkoutRequired` = value.getBoolean("checkoutRequired"),
            `confirmationBody` = value.getString("confirmationBody"),
            `confirmationTitle` = value.getString("confirmationTitle"),
            `createsSubscription` = value.getBoolean("createsSubscription"),
            `currency` = value.getString("currency"),
            `followingPeriodDays` = value.getInt("followingPeriodDays"),
            `followingPriceKopeks` = value.getInt("followingPriceKopeks"),
            `grantsAccessOnConfirmation` = value.getBoolean("grantsAccessOnConfirmation"),
            `offerId` = value.getString("offerId"),
            `planCode` = value.getString("planCode"),
            `planName` = value.getString("planName")
        )
    }
}
