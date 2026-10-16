// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandContext3(
    val `customServer`: Boolean,
    val `enabled`: Boolean,
    val `networkAvailable`: Boolean,
    val `phase`: BusinessCommandPhase9,
    val `profileFresh`: Boolean,
    val `refreshRequested`: Boolean,
    val `requiredCredits`: Long
)
