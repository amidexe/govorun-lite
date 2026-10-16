// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandProfileActivityEvent { val `kind`: String }
data class BusinessCommandProfileActivityEvent_inspect(
    val `reason`: BusinessCommandProfileRefreshReason
) : BusinessCommandProfileActivityEvent { override val `kind`: String = "inspect" }
data class BusinessCommandProfileActivityEvent_request(
    val `ancillary`: Boolean,
    val `reason`: BusinessCommandProfileRefreshReason
) : BusinessCommandProfileActivityEvent { override val `kind`: String = "request" }
data object BusinessCommandProfileActivityEvent_confirmed : BusinessCommandProfileActivityEvent { override val `kind`: String = "confirmed" }
data class BusinessCommandProfileActivityEvent_profileLoaded(
    val `ticket`: Long
) : BusinessCommandProfileActivityEvent { override val `kind`: String = "profileLoaded" }
data class BusinessCommandProfileActivityEvent_finished(
    val `succeeded`: Boolean,
    val `ticket`: Long
) : BusinessCommandProfileActivityEvent { override val `kind`: String = "finished" }
