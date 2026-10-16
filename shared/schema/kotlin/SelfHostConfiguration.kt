// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class SelfHostConfiguration(
    val `endpoint`: String,
    val `model`: String,
    val `prompt`: String,
    val `provider`: SelfHostProvider,
    val `proxyEnabled`: Boolean,
    val `proxyURL`: String
)
