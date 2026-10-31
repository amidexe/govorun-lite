// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerPromoFeature(
    val `icon`: String,
    val `title`: String,
    val `detail`: String = ""
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("detail", `detail`)
        .put("icon", `icon`)
        .put("title", `title`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerPromoFeature = ConsumerPromoFeature(
            `detail` = value.getString("detail"),
            `icon` = value.getString("icon"),
            `title` = value.getString("title")
        )
    }
}
