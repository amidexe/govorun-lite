// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandDeviceDay(
    val `date`: String,
    val `recognitionErrors`: Long,
    val `recognitionRtfBuckets`: List<Long>,
    val `recognitions`: Long,
    val `revision`: Long,
    val `speechSeconds`: Long,
    val `styleAttempts`: Long,
    val `styleFailures`: Long,
    val `styleSlow`: Long,
    val `words`: Long
)
