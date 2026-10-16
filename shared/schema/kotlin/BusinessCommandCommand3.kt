// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand3 { val `kind`: String }
data object BusinessCommandCommand3_availability : BusinessCommandCommand3 { override val `kind`: String = "availability" }
data class BusinessCommandCommand3_reliability(
    val `body`: BusinessCommandBatch
) : BusinessCommandCommand3 { override val `kind`: String = "reliability" }
data class BusinessCommandCommand3_start(
    val `body`: Any?,
    val `request_id`: String
) : BusinessCommandCommand3 { override val `kind`: String = "start" }
data class BusinessCommandCommand3_status(
    val `token`: String
) : BusinessCommandCommand3 { override val `kind`: String = "status" }
data class BusinessCommandCommand3_reconcile(
    val `passes`: List<String>
) : BusinessCommandCommand3 { override val `kind`: String = "reconcile" }
data class BusinessCommandCommand3_retire(
    val `passes`: List<String>
) : BusinessCommandCommand3 { override val `kind`: String = "retire" }
data class BusinessCommandCommand3_outcome(
    val `latency_bucket`: String,
    val `outcome`: String,
    val `pass`: String
) : BusinessCommandCommand3 { override val `kind`: String = "outcome" }
