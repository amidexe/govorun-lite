// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand14 { val `kind`: String }
data class BusinessCommandCommand14_onlineProtocol(
    val `command`: BusinessCommandCommand15
) : BusinessCommandCommand14 { override val `kind`: String = "onlineProtocol" }
data class BusinessCommandCommand14_outage(
    val `event`: BusinessCommandEvent4,
    val `now_millis`: Long,
    val `state`: BusinessCommandState4?
) : BusinessCommandCommand14 { override val `kind`: String = "outage" }
data class BusinessCommandCommand14_recording(
    val `facts`: BusinessCommandFacts2
) : BusinessCommandCommand14 { override val `kind`: String = "recording" }
data class BusinessCommandCommand14_selfHosted(
    val `command`: BusinessCommandCommand16
) : BusinessCommandCommand14 { override val `kind`: String = "selfHosted" }
data class BusinessCommandCommand14_transcript(
    val `deferred`: Boolean,
    val `event`: BusinessCommandEvent5,
    val `state`: BusinessCommandState5?
) : BusinessCommandCommand14 { override val `kind`: String = "transcript" }
data class BusinessCommandCommand14_delivery(
    val `command`: BusinessCommandCommand17
) : BusinessCommandCommand14 { override val `kind`: String = "delivery" }
data class BusinessCommandCommand14_workflow(
    val `event`: BusinessCommandEvent6,
    val `state`: BusinessCommandState6?
) : BusinessCommandCommand14 { override val `kind`: String = "workflow" }
data class BusinessCommandCommand14_deadline(
    val `characters`: Int
) : BusinessCommandCommand14 { override val `kind`: String = "deadline" }
data object BusinessCommandCommand14_constants : BusinessCommandCommand14 { override val `kind`: String = "constants" }
data class BusinessCommandCommand14_plan(
    val `settings`: BusinessCommandSettings,
    val `units`: List<Int>
) : BusinessCommandCommand14 { override val `kind`: String = "plan" }
data class BusinessCommandCommand14_admit(
    val `access`: BusinessCommandAccess,
    val `plan`: BusinessCommandPlan
) : BusinessCommandCommand14 { override val `kind`: String = "admit" }
data class BusinessCommandCommand14_response(
    val `input`: List<Int>,
    val `output`: List<Int>
) : BusinessCommandCommand14 { override val `kind`: String = "response" }
