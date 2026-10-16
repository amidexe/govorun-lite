// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent8 { val `kind`: String }
data class BusinessCommandEvent8_begin(
    val `remaining_millis`: Long
) : BusinessCommandEvent8 { override val `kind`: String = "begin" }
data class BusinessCommandEvent8_response(
    val `body`: String,
    val `now_millis`: Long,
    val `remaining_millis`: Long,
    val `retry_after`: String?,
    val `status`: Int
) : BusinessCommandEvent8 { override val `kind`: String = "response" }
data class BusinessCommandEvent8_transport(
    val `failure`: BusinessCommandFailure2,
    val `remaining_millis`: Long
) : BusinessCommandEvent8 { override val `kind`: String = "transport" }
