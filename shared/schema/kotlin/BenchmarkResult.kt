// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BenchmarkResult(
    val `cold`: BenchmarkTiming,
    val `loadMs`: Double,
    val `longAudioMs`: Double,
    val `longer`: BenchmarkTiming,
    val `model`: String,
    val `platform`: String,
    val `requestedBackend`: String,
    val `shortAudioMs`: Double,
    val `suite`: String,
    val `warm`: BenchmarkTiming
)
