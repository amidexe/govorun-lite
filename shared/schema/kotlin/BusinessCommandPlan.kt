// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandPlan(
    val `characters`: Int,
    val `deadlineMillis`: Long,
    val `length`: BusinessCommandLength,
    val `prepare`: Boolean,
    val `reason`: String,
    val `requiredCredits`: Int,
    val `route`: BusinessCommandRoute,
    val `termsRequired`: Boolean
)
