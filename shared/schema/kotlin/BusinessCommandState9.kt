// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandState9(
    val `clock`: Long,
    val `epoch`: Long,
    val `leases`: Map<String, BusinessCommandLease>,
    val `scope`: Long?,
    val `version`: Long
)
