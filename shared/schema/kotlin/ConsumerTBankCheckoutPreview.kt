// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerTBankCheckoutPreview(
    val `clientCheckoutPreview`: ConsumerCheckoutPreviewAction? = null,
    val `paymentMethodKind`: String = "card",
    val `availablePaymentMethods`: List<String> = listOf("card"),
    val `checkoutAvailable`: Boolean = true,
    val `catalogVersion`: String = "test",
    val `planCode`: String,
    val `offerId`: String = "$planCode.monthly",
    val `purchaseKind`: String,
    val `trialDays`: Int,
    val `freeAccessSource`: String,
    val `freeAccessEndsAt`: String,
    val `firstChargeAt`: String,
    val `priceKopeks`: Int,
    val `periodDays`: Int,
    val `currency`: String,
    val `serverConfirmationRequired`: Boolean,
    val `receiptContactState`: String,
    val `receiptEmailMasked`: String = "",
    val `receiptEmail`: String = "",
    val `receiptContactToken`: String = "",
    val `amountDueNowKopeks`: Int,
    val `firstChargeAmountKopeks`: Int,
    val `renewalChargeAt`: String,
    val `renewalAmountKopeks`: Int,
    val `automaticRenewal`: Boolean,
    val `cancellationMethod`: String,
    val `subscriptionConsentInitiallyAccepted`: Boolean,
    val `subscriptionConsentToken`: String,
    val `legalDocuments`: ConsumerTBankLegalDocuments,
    val `promotion`: ConsumerPromoOffer? = null,
    val `fromPlanCode`: String = "",
    val `checkoutIntentToken`: String = "",
    val `servicePeriodEndsAt`: String = "",
    val `firstChargeDelaySeconds`: Int? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("amountDueNowKopeks", `amountDueNowKopeks`)
        .put("automaticRenewal", `automaticRenewal`)
        .put("availablePaymentMethods", org.json.JSONArray(`availablePaymentMethods`.map { item -> item }))
        .put("cancellationMethod", `cancellationMethod`)
        .put("catalogVersion", `catalogVersion`)
        .put("checkoutAvailable", `checkoutAvailable`)
        .put("checkoutIntentToken", `checkoutIntentToken`)
        .put("clientCheckoutPreview", `clientCheckoutPreview`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("currency", `currency`)
        .put("firstChargeAmountKopeks", `firstChargeAmountKopeks`)
        .put("firstChargeAt", `firstChargeAt`)
        .put("firstChargeDelaySeconds", `firstChargeDelaySeconds`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("freeAccessEndsAt", `freeAccessEndsAt`)
        .put("freeAccessSource", `freeAccessSource`)
        .put("fromPlanCode", `fromPlanCode`)
        .put("legalDocuments", `legalDocuments`.toJson())
        .put("offerId", `offerId`)
        .put("paymentMethodKind", `paymentMethodKind`)
        .put("periodDays", `periodDays`)
        .put("planCode", `planCode`)
        .put("priceKopeks", `priceKopeks`)
        .put("promotion", `promotion`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("purchaseKind", `purchaseKind`)
        .put("receiptContactState", `receiptContactState`)
        .put("receiptContactToken", `receiptContactToken`)
        .put("receiptEmail", `receiptEmail`)
        .put("receiptEmailMasked", `receiptEmailMasked`)
        .put("renewalAmountKopeks", `renewalAmountKopeks`)
        .put("renewalChargeAt", `renewalChargeAt`)
        .put("serverConfirmationRequired", `serverConfirmationRequired`)
        .put("servicePeriodEndsAt", `servicePeriodEndsAt`)
        .put("subscriptionConsentInitiallyAccepted", `subscriptionConsentInitiallyAccepted`)
        .put("subscriptionConsentToken", `subscriptionConsentToken`)
        .put("trialDays", `trialDays`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerTBankCheckoutPreview = ConsumerTBankCheckoutPreview(
            `amountDueNowKopeks` = value.getInt("amountDueNowKopeks"),
            `automaticRenewal` = value.getBoolean("automaticRenewal"),
            `availablePaymentMethods` = value.getJSONArray("availablePaymentMethods").let { values -> List(values.length()) { index -> values.getString(index) } },
            `cancellationMethod` = value.getString("cancellationMethod"),
            `catalogVersion` = value.getString("catalogVersion"),
            `checkoutAvailable` = value.getBoolean("checkoutAvailable"),
            `checkoutIntentToken` = value.getString("checkoutIntentToken"),
            `clientCheckoutPreview` = if (value.isNull("clientCheckoutPreview")) null else ConsumerCheckoutPreviewAction.fromJson(value.getJSONObject("clientCheckoutPreview")),
            `currency` = value.getString("currency"),
            `firstChargeAmountKopeks` = value.getInt("firstChargeAmountKopeks"),
            `firstChargeAt` = value.getString("firstChargeAt"),
            `firstChargeDelaySeconds` = if (value.isNull("firstChargeDelaySeconds")) null else value.getInt("firstChargeDelaySeconds"),
            `freeAccessEndsAt` = value.getString("freeAccessEndsAt"),
            `freeAccessSource` = value.getString("freeAccessSource"),
            `fromPlanCode` = value.getString("fromPlanCode"),
            `legalDocuments` = ConsumerTBankLegalDocuments.fromJson(value.getJSONObject("legalDocuments")),
            `offerId` = value.getString("offerId"),
            `paymentMethodKind` = value.getString("paymentMethodKind"),
            `periodDays` = value.getInt("periodDays"),
            `planCode` = value.getString("planCode"),
            `priceKopeks` = value.getInt("priceKopeks"),
            `promotion` = if (value.isNull("promotion")) null else ConsumerPromoOffer.fromJson(value.getJSONObject("promotion")),
            `purchaseKind` = value.getString("purchaseKind"),
            `receiptContactState` = value.getString("receiptContactState"),
            `receiptContactToken` = value.getString("receiptContactToken"),
            `receiptEmail` = value.getString("receiptEmail"),
            `receiptEmailMasked` = value.getString("receiptEmailMasked"),
            `renewalAmountKopeks` = value.getInt("renewalAmountKopeks"),
            `renewalChargeAt` = value.getString("renewalChargeAt"),
            `serverConfirmationRequired` = value.getBoolean("serverConfirmationRequired"),
            `servicePeriodEndsAt` = value.getString("servicePeriodEndsAt"),
            `subscriptionConsentInitiallyAccepted` = value.getBoolean("subscriptionConsentInitiallyAccepted"),
            `subscriptionConsentToken` = value.getString("subscriptionConsentToken"),
            `trialDays` = value.getInt("trialDays")
        )
    }
}
