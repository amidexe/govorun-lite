// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState11(
    val `attempts`: Int,
    val `clock`: Long,
    val `epoch`: Long,
    val `explicitDemand`: Boolean,
    val `freshFollowup`: Boolean,
    val `localNotBefore`: Long,
    val `pendingDemand`: Boolean,
    val `phase`: BusinessCommandPhase8,
    val `requested`: Boolean,
    val `serverNotBefore`: Long,
    val `ticket`: Long
)
