// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandOverviewFacts(
    val `catalogTrialDays`: Int?,
    val `checkoutPending`: Boolean,
    val `epoch`: Long,
    val `now`: String,
    val `paymentUpdating`: Boolean,
    val `signedIn`: Boolean,
    val `statusFresh`: Boolean,
    val `statusUpdating`: Boolean,
    val `subscription`: BusinessCommandSubscriptionFacts?
)
