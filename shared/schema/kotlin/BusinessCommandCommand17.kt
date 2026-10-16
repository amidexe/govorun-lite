// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand17 { val `kind`: String }
data object BusinessCommandCommand17_constants : BusinessCommandCommand17 { override val `kind`: String = "constants" }
data class BusinessCommandCommand17_prepare(
    val `fallback`: Boolean,
    val `has_outcome_pass`: Boolean,
    val `latency_known`: Boolean,
    val `marker`: BusinessCommandMarker,
    val `original`: List<Int>,
    val `text`: List<Int>
) : BusinessCommandCommand17 { override val `kind`: String = "prepare" }
data class BusinessCommandCommand17_stream(
    val `event`: BusinessCommandStreamEvent,
    val `marker`: BusinessCommandMarker,
    val `state`: BusinessCommandStreamState?
) : BusinessCommandCommand17 { override val `kind`: String = "stream" }
