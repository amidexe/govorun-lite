// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingCatalog(
    val `schemaVersion`: Int = 0,
    val `catalogVersion`: String = "",
    val `publishedAt`: String = "",
    val `refreshAfterSeconds`: Int = 0,
    val `currency`: String = "RUB",
    val `ruStoreEnabled`: Boolean = false,
    val `tBankEnabled`: Boolean = false,
    val `tBankPreviewEnabled`: Boolean = tBankEnabled,
    val `trialOffer`: ConsumerBillingTrialOffer = ConsumerBillingTrialOffer(`checkoutOpenGrantsAccess` = false, `durationDays` = 0, `eligible` = false, `requiresLocalReserveEmpty` = false, `requiresVerifiedPaymentMethod` = true, `state` = "disabled"),
    val `starterGrant`: ConsumerStarterGrantOffer = ConsumerStarterGrantOffer(`clientOfferVisible` = false, `creditLimit` = 0, `expires` = false, `requiresAuthentication` = true, `requiresCode` = false, `requiresPaymentMethod` = false, `state` = "disabled"),
    val `plans`: List<ConsumerBillingPlan> = listOf(),
    val `storefront`: ConsumerBillingStorefront = ConsumerBillingStorefront(`message` = "", `state` = "open"),
    val `programControlVersion`: String = "",
    val `promoProgram`: ConsumerPromoProgram = ConsumerPromoProgram(`accessDays` = 0, `accessEndsAt` = null, `accessMode` = "common_end", `activationAvailable` = false, `campaignId` = "", `canActivate` = false, `dailyLimit` = 0, `endsAt` = null, `enrollmentOpen` = false, `message` = "", `participated` = false, `periodDays` = 0, `periodLimit` = 0, `programVersion` = "", `state` = "disabled", `visible` = false),
    val `clientTrialDays`: Int = 0
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("catalogVersion", `catalogVersion`)
        .put("clientTrialDays", `clientTrialDays`)
        .put("currency", `currency`)
        .put("plans", org.json.JSONArray(`plans`.map { item -> item.toJson() }))
        .put("programControlVersion", `programControlVersion`)
        .put("promoProgram", `promoProgram`.toJson())
        .put("publishedAt", `publishedAt`)
        .put("refreshAfterSeconds", `refreshAfterSeconds`)
        .put("ruStoreEnabled", `ruStoreEnabled`)
        .put("schemaVersion", `schemaVersion`)
        .put("starterGrant", `starterGrant`.toJson())
        .put("storefront", `storefront`.toJson())
        .put("tBankEnabled", `tBankEnabled`)
        .put("tBankPreviewEnabled", `tBankPreviewEnabled`)
        .put("trialOffer", `trialOffer`.toJson())
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingCatalog = ConsumerBillingCatalog(
            `catalogVersion` = value.getString("catalogVersion"),
            `clientTrialDays` = value.getInt("clientTrialDays"),
            `currency` = value.getString("currency"),
            `plans` = value.getJSONArray("plans").let { values -> List(values.length()) { index -> ConsumerBillingPlan.fromJson(values.getJSONObject(index)) } },
            `programControlVersion` = value.getString("programControlVersion"),
            `promoProgram` = ConsumerPromoProgram.fromJson(value.getJSONObject("promoProgram")),
            `publishedAt` = value.getString("publishedAt"),
            `refreshAfterSeconds` = value.getInt("refreshAfterSeconds"),
            `ruStoreEnabled` = value.getBoolean("ruStoreEnabled"),
            `schemaVersion` = value.getInt("schemaVersion"),
            `starterGrant` = ConsumerStarterGrantOffer.fromJson(value.getJSONObject("starterGrant")),
            `storefront` = ConsumerBillingStorefront.fromJson(value.getJSONObject("storefront")),
            `tBankEnabled` = value.getBoolean("tBankEnabled"),
            `tBankPreviewEnabled` = value.getBoolean("tBankPreviewEnabled"),
            `trialOffer` = ConsumerBillingTrialOffer.fromJson(value.getJSONObject("trialOffer"))
        )
    }
}
