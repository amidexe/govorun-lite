// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandKey(
    val `algorithm`: String,
    val `keyId`: String,
    val `notAfterMillis`: Long,
    val `notBeforeMillis`: Long,
    val `publicKey`: String
)
