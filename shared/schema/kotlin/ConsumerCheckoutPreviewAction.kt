// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerCheckoutPreviewAction(
    val `canConfirm`: Boolean,
    val `confirmationPath`: String,
    val `receiptContactRequired`: Boolean
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("canConfirm", `canConfirm`)
        .put("confirmationPath", `confirmationPath`)
        .put("receiptContactRequired", `receiptContactRequired`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerCheckoutPreviewAction = ConsumerCheckoutPreviewAction(
            `canConfirm` = value.getBoolean("canConfirm"),
            `confirmationPath` = value.getString("confirmationPath"),
            `receiptContactRequired` = value.getBoolean("receiptContactRequired")
        )
    }
}
