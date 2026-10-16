// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandSnapshot(
    val `cursor`: Long,
    val `days`: List<BusinessCommandDay>,
    val `generation`: Long,
    val `total`: BusinessCommandTotal?
)
