// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandProfileActivity(
    val `cadence`: BusinessCommandProfileRefreshCadence,
    val `flight`: BusinessCommandFlight?,
    val `generation`: Long?,
    val `sequence`: Long
)
