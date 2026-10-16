// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandStreamEvent { val `kind`: String }
data class BusinessCommandStreamEvent_fragment(
    val `units`: List<Int>
) : BusinessCommandStreamEvent { override val `kind`: String = "fragment" }
data class BusinessCommandStreamEvent_flush(
    val `had_text`: Boolean
) : BusinessCommandStreamEvent { override val `kind`: String = "flush" }
data class BusinessCommandStreamEvent_suffix(
    val `had_text`: Boolean
) : BusinessCommandStreamEvent { override val `kind`: String = "suffix" }
