// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerCheckoutPresentation(
    val `disposition`: String,
    val `canCancel`: Boolean,
    val `resumeUrl`: String? = null,
    val `expiresInSeconds`: Int? = null,
    val `message`: String,
    val `accessRefreshPending`: Boolean = false,
    val `launchAction`: String? = null
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("accessRefreshPending", `accessRefreshPending`)
        .put("canCancel", `canCancel`)
        .put("disposition", `disposition`)
        .put("expiresInSeconds", `expiresInSeconds`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("launchAction", `launchAction`?.let { item -> item } ?: org.json.JSONObject.NULL)
        .put("message", `message`)
        .put("resumeUrl", `resumeUrl`?.let { item -> item } ?: org.json.JSONObject.NULL)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerCheckoutPresentation = ConsumerCheckoutPresentation(
            `accessRefreshPending` = value.getBoolean("accessRefreshPending"),
            `canCancel` = value.getBoolean("canCancel"),
            `disposition` = value.getString("disposition"),
            `expiresInSeconds` = if (value.isNull("expiresInSeconds")) null else value.getInt("expiresInSeconds"),
            `launchAction` = if (value.isNull("launchAction")) null else value.getString("launchAction"),
            `message` = value.getString("message"),
            `resumeUrl` = if (value.isNull("resumeUrl")) null else value.getString("resumeUrl")
        )
    }
}
