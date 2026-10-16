// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand8 { val `kind`: String }
data class BusinessCommandCommand8_enrollment(
    val `content`: BusinessCommandDocument,
    val `history`: Boolean,
    val `other_account_history`: Boolean,
    val `remote_data`: Boolean?,
    val `requested`: Boolean,
    val `statistics`: BusinessCommandLocalStatistics,
    val `stored`: BusinessCommandMode?
) : BusinessCommandCommand8 { override val `kind`: String = "enrollment" }
data class BusinessCommandCommand8_start(
    val `scope`: BusinessCommandScope2
) : BusinessCommandCommand8 { override val `kind`: String = "start" }
data class BusinessCommandCommand8_advance(
    val `flow`: BusinessCommandFlow2,
    val `scope`: BusinessCommandScope2?,
    val `stage`: BusinessCommandStage2,
    val `succeeded`: Boolean
) : BusinessCommandCommand8 { override val `kind`: String = "advance" }
data class BusinessCommandCommand8_schedule(
    val `failures`: Int,
    val `jitter`: Double,
    val `pending`: Boolean,
    val `succeeded`: Boolean
) : BusinessCommandCommand8 { override val `kind`: String = "schedule" }
