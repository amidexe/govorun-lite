// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent4 { val `kind`: String }
data object BusinessCommandEvent4_beginRequest : BusinessCommandEvent4 { override val `kind`: String = "beginRequest" }
data class BusinessCommandEvent4_deadline(
    val `normal`: Long
) : BusinessCommandEvent4 { override val `kind`: String = "deadline" }
data class BusinessCommandEvent4_finishRequest(
    val `cancelled`: Boolean,
    val `failure`: BusinessCommandFailure,
    val `jitter`: Double,
    val `retry_after`: Long,
    val `retry_hints`: List<BusinessCommandRetryHint>,
    val `ticket`: Long
) : BusinessCommandEvent4 { override val `kind`: String = "finishRequest" }
data object BusinessCommandEvent4_demand : BusinessCommandEvent4 { override val `kind`: String = "demand" }
data object BusinessCommandEvent4_recording : BusinessCommandEvent4 { override val `kind`: String = "recording" }
data object BusinessCommandEvent4_nextProbe : BusinessCommandEvent4 { override val `kind`: String = "nextProbe" }
data class BusinessCommandEvent4_finishProbe(
    val `elapsed`: Long,
    val `healthy`: Boolean,
    val `jitter`: Double,
    val `retry_after`: Long,
    val `retry_hints`: List<BusinessCommandRetryHint>,
    val `sample_age`: Long,
    val `ticket`: Long
) : BusinessCommandEvent4 { override val `kind`: String = "finishProbe" }
data class BusinessCommandEvent4_abandonProbe(
    val `ticket`: Long
) : BusinessCommandEvent4 { override val `kind`: String = "abandonProbe" }
