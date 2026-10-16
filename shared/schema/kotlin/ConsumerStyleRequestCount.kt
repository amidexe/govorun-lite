// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerStyleRequestCount(
    val `count`: Int,
    val `credits`: Int
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("count", `count`)
        .put("credits", `credits`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerStyleRequestCount = ConsumerStyleRequestCount(
            `count` = value.getInt("count"),
            `credits` = value.getInt("credits")
        )
    }
}
