// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoBillingPeriod(
    val `number`: Int,
    val `standardPriceKopeks`: Int,
    val `payablePriceKopeks`: Int,
    val `discountAmountKopeks`: Int
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("discountAmountKopeks", `discountAmountKopeks`)
        .put("number", `number`)
        .put("payablePriceKopeks", `payablePriceKopeks`)
        .put("standardPriceKopeks", `standardPriceKopeks`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoBillingPeriod = ConsumerPromoBillingPeriod(
            `discountAmountKopeks` = value.getInt("discountAmountKopeks"),
            `number` = value.getInt("number"),
            `payablePriceKopeks` = value.getInt("payablePriceKopeks"),
            `standardPriceKopeks` = value.getInt("standardPriceKopeks")
        )
    }
}
