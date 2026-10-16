// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingRefundRequest(
    val `id`: String,
    val `state`: String,
    val `reasonCode`: String,
    val `requestedAt`: String,
    val `decisionCode`: String = "",
    val `decisionMode`: String = "",
    val `approvedAmountKopeks`: Int = 0,
    val `currency`: String = "RUB",
    val `renewalStopped`: Boolean = true,
    val `currentAccessEndsAt`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("approvedAmountKopeks", `approvedAmountKopeks`)
        .put("currency", `currency`)
        .put("currentAccessEndsAt", `currentAccessEndsAt`)
        .put("decisionCode", `decisionCode`)
        .put("decisionMode", `decisionMode`)
        .put("id", `id`)
        .put("reasonCode", `reasonCode`)
        .put("renewalStopped", `renewalStopped`)
        .put("requestedAt", `requestedAt`)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingRefundRequest = ConsumerBillingRefundRequest(
            `approvedAmountKopeks` = value.getInt("approvedAmountKopeks"),
            `currency` = value.getString("currency"),
            `currentAccessEndsAt` = value.getString("currentAccessEndsAt"),
            `decisionCode` = value.getString("decisionCode"),
            `decisionMode` = value.getString("decisionMode"),
            `id` = value.getString("id"),
            `reasonCode` = value.getString("reasonCode"),
            `renewalStopped` = value.getBoolean("renewalStopped"),
            `requestedAt` = value.getString("requestedAt"),
            `state` = value.getString("state")
        )
    }
}
