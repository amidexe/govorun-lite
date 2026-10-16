// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand11 { val `kind`: String }
data class BusinessCommandCommand11_accept(
    val `current`: BusinessCommandSnapshot,
    val `response`: BusinessCommandResponse
) : BusinessCommandCommand11 { override val `kind`: String = "accept" }
data class BusinessCommandCommand11_prepare(
    val `acknowledged`: Map<String, Long>,
    val `local_days`: List<BusinessCommandDeviceDay>,
    val `revisions`: Map<String, Long>,
    val `today`: String
) : BusinessCommandCommand11 { override val `kind`: String = "prepare" }
