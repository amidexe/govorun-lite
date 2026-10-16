// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandRequestSnapshot(
    val `blindedMessage`: String,
    val `encodedMessage`: String,
    val `exponent`: String,
    val `inverse`: String,
    val `keyId`: String,
    val `keyNotAfterMillis`: Long,
    val `keyNotBeforeMillis`: Long,
    val `modulus`: String,
    val `preparedMessage`: String
)
