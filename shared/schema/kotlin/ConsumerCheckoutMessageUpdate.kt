// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerCheckoutMessageUpdate(
    val `replace`: Boolean = false,
    val `message`: String? = null,
    val `refreshStatus`: Boolean = false,
    val `refreshAccess`: Boolean = false
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("message", `message`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("refreshAccess", `refreshAccess`)
        .put("refreshStatus", `refreshStatus`)
        .put("replace", `replace`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerCheckoutMessageUpdate = ConsumerCheckoutMessageUpdate(
            `message` = if (value.isNull("message")) null else value.getString("message"),
            `refreshAccess` = value.getBoolean("refreshAccess"),
            `refreshStatus` = value.getBoolean("refreshStatus"),
            `replace` = value.getBoolean("replace")
        )
    }
}
