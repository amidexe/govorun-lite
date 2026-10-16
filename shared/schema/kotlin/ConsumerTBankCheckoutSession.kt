// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerTBankCheckoutSession(
    val `paymentMethodKind`: String = "card",
    val `catalogVersion`: String = "test",
    val `checkoutSessionId`: String,
    val `state`: String,
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
    val `browserReturnGrantsAccess`: Boolean,
    val `serverConfirmationRequired`: Boolean,
    val `paymentUrl`: String = "",
    val `firstChargeDelaySeconds`: Int? = null,
    val `canRestartBinding`: Boolean = false,
    val `cancellationPending`: Boolean = false,
    val `expiresInSeconds`: Int? = null,
    val `clientCheckout`: ConsumerCheckoutPresentation? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("browserReturnGrantsAccess", `browserReturnGrantsAccess`)
        .put("canRestartBinding", `canRestartBinding`)
        .put("cancellationPending", `cancellationPending`)
        .put("catalogVersion", `catalogVersion`)
        .put("checkoutSessionId", `checkoutSessionId`)
        .put("clientCheckout", `clientCheckout`?.let { item -> item.toJson() } ?: org.json.JSONObject.NULL)
        .put("currency", `currency`)
        .put("expiresInSeconds", `expiresInSeconds`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("firstChargeAt", `firstChargeAt`)
        .put("firstChargeDelaySeconds", `firstChargeDelaySeconds`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("freeAccessEndsAt", `freeAccessEndsAt`)
        .put("freeAccessSource", `freeAccessSource`)
        .put("offerId", `offerId`)
        .put("paymentMethodKind", `paymentMethodKind`)
        .put("paymentUrl", `paymentUrl`)
        .put("periodDays", `periodDays`)
        .put("planCode", `planCode`)
        .put("priceKopeks", `priceKopeks`)
        .put("purchaseKind", `purchaseKind`)
        .put("serverConfirmationRequired", `serverConfirmationRequired`)
        .put("state", `state`)
        .put("trialDays", `trialDays`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerTBankCheckoutSession = ConsumerTBankCheckoutSession(
            `browserReturnGrantsAccess` = value.getBoolean("browserReturnGrantsAccess"),
            `canRestartBinding` = value.getBoolean("canRestartBinding"),
            `cancellationPending` = value.getBoolean("cancellationPending"),
            `catalogVersion` = value.getString("catalogVersion"),
            `checkoutSessionId` = value.getString("checkoutSessionId"),
            `clientCheckout` = if (value.isNull("clientCheckout")) null else ConsumerCheckoutPresentation.fromJson(value.getJSONObject("clientCheckout")),
            `currency` = value.getString("currency"),
            `expiresInSeconds` = if (value.isNull("expiresInSeconds")) null else value.getInt("expiresInSeconds"),
            `firstChargeAt` = value.getString("firstChargeAt"),
            `firstChargeDelaySeconds` = if (value.isNull("firstChargeDelaySeconds")) null else value.getInt("firstChargeDelaySeconds"),
            `freeAccessEndsAt` = value.getString("freeAccessEndsAt"),
            `freeAccessSource` = value.getString("freeAccessSource"),
            `offerId` = value.getString("offerId"),
            `paymentMethodKind` = value.getString("paymentMethodKind"),
            `paymentUrl` = value.getString("paymentUrl"),
            `periodDays` = value.getInt("periodDays"),
            `planCode` = value.getString("planCode"),
            `priceKopeks` = value.getInt("priceKopeks"),
            `purchaseKind` = value.getString("purchaseKind"),
            `serverConfirmationRequired` = value.getBoolean("serverConfirmationRequired"),
            `state` = value.getString("state"),
            `trialDays` = value.getInt("trialDays")
        )
    }
}
