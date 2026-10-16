// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingPaymentMethod(
    val `state`: String = "unknown",
    val `lastFour`: String? = null,
    val `canRemove`: Boolean = false,
    val `kind`: String = "card",
    val `available`: Boolean? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("available", `available`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("canRemove", `canRemove`)
        .put("kind", `kind`)
        .put("lastFour", `lastFour`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("state", `state`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingPaymentMethod = ConsumerBillingPaymentMethod(
            `available` = if (value.isNull("available")) null else value.getBoolean("available"),
            `canRemove` = value.getBoolean("canRemove"),
            `kind` = value.getString("kind"),
            `lastFour` = if (value.isNull("lastFour")) null else value.getString("lastFour"),
            `state` = value.getString("state")
        )
    }
}
