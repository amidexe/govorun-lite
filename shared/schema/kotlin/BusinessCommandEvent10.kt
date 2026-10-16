// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent10 { val `kind`: String }
data class BusinessCommandEvent10_request(
    val `delay_millis`: Long,
    val `join`: Boolean,
    val `trigger`: BusinessCommandTrigger
) : BusinessCommandEvent10 { override val `kind`: String = "request" }
data class BusinessCommandEvent10_complete(
    val `outcome`: BusinessCommandOutcome,
    val `retry_after_millis`: Long?,
    val `ticket`: Long,
    val `wire_attempted`: Boolean
) : BusinessCommandEvent10 { override val `kind`: String = "complete" }
data class BusinessCommandEvent10_tick(
    val `ticket`: Long
) : BusinessCommandEvent10 { override val `kind`: String = "tick" }
data object BusinessCommandEvent10_cancel : BusinessCommandEvent10 { override val `kind`: String = "cancel" }
