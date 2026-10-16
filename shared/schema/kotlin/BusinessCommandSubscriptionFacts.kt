// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandSubscriptionFacts(
    val `currentPeriodEndsAt`: String?,
    val `nextCharge`: BusinessCommandNextChargeFacts?,
    val `paymentMethod`: BusinessCommandPaymentMethodFacts?,
    val `renewalIssue`: BusinessCommandRenewalIssueFacts?,
    val `renewsAutomatically`: Boolean,
    val `state`: String
)
