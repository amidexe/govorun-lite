// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BenchmarkDevice(
    val `backend`: String,
    val `failureCode`: String?,
    val `failureDetail`: String?,
    val `id`: String,
    val `name`: String,
    val `result`: BenchmarkResult?,
    val `stage`: String,
    val `state`: String
)
