// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerBillingCheckoutPresentation(
    val `settled`: ConsumerCheckoutMessageUpdate = ConsumerCheckoutMessageUpdate(`message` = null, `refreshAccess` = false, `refreshStatus` = false, `replace` = false),
    val `scheduled`: ConsumerCheckoutMessageUpdate = ConsumerCheckoutMessageUpdate(`message` = null, `refreshAccess` = false, `refreshStatus` = true, `replace` = false)
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("scheduled", `scheduled`.toJson())
        .put("settled", `settled`.toJson())
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerBillingCheckoutPresentation = ConsumerBillingCheckoutPresentation(
            `scheduled` = ConsumerCheckoutMessageUpdate.fromJson(value.getJSONObject("scheduled")),
            `settled` = ConsumerCheckoutMessageUpdate.fromJson(value.getJSONObject("settled"))
        )
    }
}
