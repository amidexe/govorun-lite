// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingNextCharge(
    val `amountKopeks`: Int,
    val `currency`: String,
    val `chargeAt`: String,
    val `noticeAt`: String
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("amountKopeks", `amountKopeks`)
        .put("chargeAt", `chargeAt`)
        .put("currency", `currency`)
        .put("noticeAt", `noticeAt`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingNextCharge = ConsumerBillingNextCharge(
            `amountKopeks` = value.getInt("amountKopeks"),
            `chargeAt` = value.getString("chargeAt"),
            `currency` = value.getString("currency"),
            `noticeAt` = value.getString("noticeAt")
        )
    }
}
