// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState4(
    val `backoff`: Int,
    val `failures`: Int,
    val `generation`: Long,
    val `lastFailure`: Long?,
    val `paused`: Boolean,
    val `trialActive`: Boolean,
    val `trialAt`: Long
)
