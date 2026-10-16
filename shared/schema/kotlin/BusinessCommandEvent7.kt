// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent7 { val `kind`: String }
data class BusinessCommandEvent7_begin(
    val `remaining_millis`: Long
) : BusinessCommandEvent7 { override val `kind`: String = "begin" }
data class BusinessCommandEvent7_response(
    val `now_millis`: Long,
    val `remaining_millis`: Long,
    val `retry_after`: String?,
    val `status`: Int
) : BusinessCommandEvent7 { override val `kind`: String = "response" }
data class BusinessCommandEvent7_transport(
    val `failure`: BusinessCommandFailure2,
    val `remaining_millis`: Long
) : BusinessCommandEvent7 { override val `kind`: String = "transport" }
data class BusinessCommandEvent7_processing(
    val `elapsed_millis`: Long,
    val `remaining_millis`: Long
) : BusinessCommandEvent7 { override val `kind`: String = "processing" }
