// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BenchmarkSnapshot(
    val `activeDeviceId`: String?,
    val `devices`: List<BenchmarkDevice>,
    val `failureCode`: String?,
    val `report`: String,
    val `stage`: String,
    val `state`: String
)
