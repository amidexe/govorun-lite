// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BillingCheckout(
    val `actionEnabled`: Boolean,
    val `actionTitle`: String,
    val `amountDueNow`: String,
    val `availablePaymentMethods`: List<String>,
    val `consent`: String,
    val `email`: String?,
    val `emailMasked`: String?,
    val `emailRequired`: Boolean,
    val `firstCharge`: String,
    val `handoff`: String,
    val `hasLegalDocuments`: Boolean,
    val `id`: String,
    val `message`: String?,
    val `paymentMethodKind`: String,
    val `planTitle`: String,
    val `receiptContactNeeded`: Boolean,
    val `receiptPurpose`: String,
    val `recurringDisclosure`: String,
    val `renewal`: String,
    val `title`: String,
    val `trialHeadline`: String
)
