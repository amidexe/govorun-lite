// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BillingPlan(
    val `current`: Boolean,
    val `id`: String,
    val `limit`: String,
    val `multiplier`: String?,
    val `period`: String,
    val `price`: String,
    val `title`: String,
    val `trial`: String
)
