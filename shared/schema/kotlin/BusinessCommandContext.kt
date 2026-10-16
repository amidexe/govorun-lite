// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandContext(
    val `builtInOptions`: Map<String, BusinessCommandOptions>,
    val `currentOptions`: BusinessCommandOptions,
    val `currentOptionsStyle`: String,
    val `state`: BusinessCommandState2
)
