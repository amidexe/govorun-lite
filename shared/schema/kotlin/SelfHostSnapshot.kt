// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class SelfHostSnapshot(
    val `busy`: Boolean,
    val `cancelling`: Boolean,
    val `configuration`: SelfHostConfiguration,
    val `configured`: Boolean,
    val `consent`: Boolean,
    val `failed`: Boolean,
    val `keyState`: SelfHostKeyState,
    val `message`: String?,
    val `operation`: String?,
    val `operationID`: String?,
    val `routeEnabled`: Boolean,
    val `scope`: String,
    val `stylingEnabled`: Boolean,
    val `verified`: Boolean
)
