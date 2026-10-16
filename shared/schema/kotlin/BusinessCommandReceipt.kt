// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandReceipt(
    val `account`: String,
    val `days`: Map<String, BusinessCommandDeviceDay>,
    val `epoch`: Long,
    val `total`: BusinessCommandTotal
)
