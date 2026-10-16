// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerTBankLegalDocument(
    val `version`: String,
    val `url`: String
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("url", `url`)
        .put("version", `version`)
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerTBankLegalDocument = ConsumerTBankLegalDocument(
            `url` = value.getString("url"),
            `version` = value.getString("version")
        )
    }
}
