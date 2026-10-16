// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState(
    val `base`: BusinessCommandDocument,
    val `checkedAt`: Long?,
    val `etag`: String,
    val `inFlight`: BusinessCommandWrite?,
    val `schemaVersion`: Int
)
