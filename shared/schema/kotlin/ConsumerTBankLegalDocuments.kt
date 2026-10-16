// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class ConsumerTBankLegalDocuments(
    val `subscriptionTerms`: ConsumerTBankLegalDocument,
    val `userAgreement`: ConsumerTBankLegalDocument,
    val `refundPolicy`: ConsumerTBankLegalDocument,
    val `privacyNotice`: ConsumerTBankLegalDocument,
    val `receiptDataConsent`: ConsumerTBankLegalDocument
) {
    fun toJson(): org.json.JSONObject = org.json.JSONObject()
        .put("privacyNotice", `privacyNotice`.toJson())
        .put("receiptDataConsent", `receiptDataConsent`.toJson())
        .put("refundPolicy", `refundPolicy`.toJson())
        .put("subscriptionTerms", `subscriptionTerms`.toJson())
        .put("userAgreement", `userAgreement`.toJson())
    companion object {
        fun fromJson(value: org.json.JSONObject): ConsumerTBankLegalDocuments = ConsumerTBankLegalDocuments(
            `privacyNotice` = ConsumerTBankLegalDocument.fromJson(value.getJSONObject("privacyNotice")),
            `receiptDataConsent` = ConsumerTBankLegalDocument.fromJson(value.getJSONObject("receiptDataConsent")),
            `refundPolicy` = ConsumerTBankLegalDocument.fromJson(value.getJSONObject("refundPolicy")),
            `subscriptionTerms` = ConsumerTBankLegalDocument.fromJson(value.getJSONObject("subscriptionTerms")),
            `userAgreement` = ConsumerTBankLegalDocument.fromJson(value.getJSONObject("userAgreement"))
        )
    }
}
