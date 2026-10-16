// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingStorefront(
    val `state`: String = "open",
    val `message`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("message", `message`)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingStorefront = ConsumerBillingStorefront(
            `message` = value.getString("message"),
            `state` = value.getString("state")
        )
    }
}
