// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingRenewalIssue(
    val `state`: String,
    val `resolutionKind`: String = "",
    val `chargeKind`: String = "",
    val `paymentState`: String = "",
    val `amountKopeks`: Int = 0,
    val `currency`: String = "",
    val `paymentMethodSaved`: Boolean = false,
    val `attemptCount`: Int,
    val `maximumAttempts`: Int,
    val `occurredAt`: String = "",
    val `nextAttemptAt`: String = "",
    val `accessEndsAt`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessEndsAt", `accessEndsAt`)
        .put("amountKopeks", `amountKopeks`)
        .put("attemptCount", `attemptCount`)
        .put("chargeKind", `chargeKind`)
        .put("currency", `currency`)
        .put("maximumAttempts", `maximumAttempts`)
        .put("nextAttemptAt", `nextAttemptAt`)
        .put("occurredAt", `occurredAt`)
        .put("paymentMethodSaved", `paymentMethodSaved`)
        .put("paymentState", `paymentState`)
        .put("resolutionKind", `resolutionKind`)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingRenewalIssue = ConsumerBillingRenewalIssue(
            `accessEndsAt` = value.getString("accessEndsAt"),
            `amountKopeks` = value.getInt("amountKopeks"),
            `attemptCount` = value.getInt("attemptCount"),
            `chargeKind` = value.getString("chargeKind"),
            `currency` = value.getString("currency"),
            `maximumAttempts` = value.getInt("maximumAttempts"),
            `nextAttemptAt` = value.getString("nextAttemptAt"),
            `occurredAt` = value.getString("occurredAt"),
            `paymentMethodSaved` = value.getBoolean("paymentMethodSaved"),
            `paymentState` = value.getString("paymentState"),
            `resolutionKind` = value.getString("resolutionKind"),
            `state` = value.getString("state")
        )
    }
}
