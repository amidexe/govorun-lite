// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand5 { val `kind`: String }
data class BusinessCommandCommand5_recording(
    val `action`: BusinessCommandRecordingAction,
    val `enabled`: Boolean,
    val `mode`: String?
) : BusinessCommandCommand5 { override val `kind`: String = "recording" }
data class BusinessCommandCommand5_enable(
    val `enabled`: Boolean
) : BusinessCommandCommand5 { override val `kind`: String = "enable" }
data object BusinessCommandCommand5_dismissTerms : BusinessCommandCommand5 { override val `kind`: String = "dismissTerms" }
data class BusinessCommandCommand5_acceptTerms(
    val `accepted_at_millis`: Long,
    val `version`: Int
) : BusinessCommandCommand5 { override val `kind`: String = "acceptTerms" }
data object BusinessCommandCommand5_reset : BusinessCommandCommand5 { override val `kind`: String = "reset" }
