// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerAnnouncement(
    val `id`: String,
    val `title`: String,
    val `text`: String = "",
    val `action`: String = "",
    val `url`: String = "",
    val `belowVersionCode`: Int = 0
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("action", `action`)
        .put("belowVersionCode", `belowVersionCode`)
        .put("id", `id`)
        .put("text", `text`)
        .put("title", `title`)
        .put("url", `url`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerAnnouncement = ConsumerAnnouncement(
            `action` = value.getString("action"),
            `belowVersionCode` = value.getInt("belowVersionCode"),
            `id` = value.getString("id"),
            `text` = value.getString("text"),
            `title` = value.getString("title"),
            `url` = value.getString("url")
        )
    }
}
