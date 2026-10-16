// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState4(
    val `backoff`: Int,
    val `freshDemand`: Boolean,
    val `generation`: Long,
    val `hasOpened`: Boolean,
    val `lastFailure`: Long?,
    val `lastRecordingProbe`: Long?,
    val `paused`: Boolean,
    val `probeActive`: Boolean,
    val `probeAt`: Long,
    val `protectedUntil`: Long,
    val `remaining`: Int
)
