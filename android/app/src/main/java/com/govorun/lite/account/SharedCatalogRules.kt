package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONArray
import org.json.JSONObject

/** Only DTO conversion lives here; the common catalog owner validates offers. */
internal fun sharedBillingCatalog(wire: JSONObject): JSONObject =
    SharedBusiness.evaluate(JSONObject().put("operation", "billingCatalog").put("catalog", wire)
        .put("channel", "direct")).getJSONObject("catalog")

internal fun validateBillingCatalog(catalog: BillingCatalog) { sharedBillingCatalog(catalog.sharedWire()) }

internal fun samePublishedCatalog(first: BillingCatalog, second: BillingCatalog): Boolean =
    SharedBusiness.evaluate(JSONObject().put("operation", "samePublishedCatalog")
        .put("first", first.sharedWire()).put("second", second.sharedWire())).getBoolean("same")

internal fun BillingCatalog.sharedWire(): JSONObject = JSONObject()
    .put("schemaVersion", schemaVersion).put("catalogVersion", catalogVersion)
    .put("publishedAt", publishedAt).put("refreshAfterSeconds", refreshAfterSeconds).put("currency", currency)
    .put("programControlVersion", programControlVersion)
    .put("channels", JSONObject().put("tbank", tBankEnabled).put("tbankPreview", tBankPreviewEnabled)
        .put("rustore", ruStoreEnabled))
    .put("storefront", JSONObject().put("state", storefront.state).put("message", storefront.message))
    .put("trialOffer", JSONObject().put("state", trialOffer.state).put("eligible", trialOffer.eligible)
        .put("durationDays", trialOffer.durationDays)
        .put("requiresVerifiedPaymentMethod", trialOffer.requiresVerifiedPaymentMethod)
        .put("checkoutOpenGrantsAccess", trialOffer.checkoutOpenGrantsAccess))
    .put("offers", JSONArray(plans.map { plan ->
        JSONObject().put("offerId", plan.offerId).put("planCode", plan.code).put("planName", plan.name)
            .put("billingPeriodLabel", plan.billingPeriodLabel).put("priceKopeks", plan.priceKopeks)
            .put("periodDays", plan.periodDays).put("dailyLimit", plan.dailyLimit).put("periodLimit", plan.periodLimit)
            .put("recommended", plan.recommended)
            .put("ruStoreProductId", plan.ruStoreProductId)
    }))
