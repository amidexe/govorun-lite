// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent11 { val `kind`: String }
data class BusinessCommandEvent11_request(
    val `join`: Boolean,
    val `trigger`: BusinessCommandTrigger2
) : BusinessCommandEvent11 { override val `kind`: String = "request" }
data class BusinessCommandEvent11_complete(
    val `outcome`: BusinessCommandOutcome2,
    val `retry_after_millis`: Long?,
    val `status`: Int?,
    val `ticket`: Long
) : BusinessCommandEvent11 { override val `kind`: String = "complete" }
data class BusinessCommandEvent11_tick(
    val `ticket`: Long
) : BusinessCommandEvent11 { override val `kind`: String = "tick" }
data object BusinessCommandEvent11_cancel : BusinessCommandEvent11 { override val `kind`: String = "cancel" }
