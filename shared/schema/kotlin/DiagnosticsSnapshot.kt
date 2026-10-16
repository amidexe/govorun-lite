// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class DiagnosticsSnapshot(
    val `events`: List<DiagnosticsRow>,
    val `extendedEnabled`: Boolean,
    val `extendedEvents`: List<DiagnosticsRow>,
    val `maximumEvents`: Int,
    val `nativeAvailable`: Boolean
)
