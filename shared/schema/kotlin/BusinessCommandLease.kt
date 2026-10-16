// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandLease(
    val `notBefore`: Long,
    val `owner`: String?,
    val `phase`: BusinessCommandPhase5,
    val `version`: Long
)
