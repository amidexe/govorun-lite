// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState10(
    val `attempts`: Int,
    val `clock`: Long,
    val `epoch`: Long,
    val `followupUsed`: Boolean,
    val `freshFollowup`: Boolean,
    val `notBefore`: Long,
    val `phase`: BusinessCommandPhase7,
    val `requested`: Boolean,
    val `ticket`: Long
)
