// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandResponse(
    val `cursor`: Long,
    val `dayWindowStart`: String,
    val `days`: List<BusinessCommandDay>,
    val `generation`: Long,
    val `schemaVersion`: Int,
    val `snapshot`: Boolean,
    val `total`: BusinessCommandTotal
)
