// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoBonus(
    val `credits`: Int,
    val `validDays`: Int
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("credits", `credits`)
        .put("validDays", `validDays`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoBonus = ConsumerPromoBonus(
            `credits` = value.getInt("credits"),
            `validDays` = value.getInt("validDays")
        )
    }
}
