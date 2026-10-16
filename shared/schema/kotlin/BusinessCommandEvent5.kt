// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent5 { val `kind`: String }
data class BusinessCommandEvent5_fragment(
    val `units`: List<Int>
) : BusinessCommandEvent5 { override val `kind`: String = "fragment" }
data object BusinessCommandEvent5_complete : BusinessCommandEvent5 { override val `kind`: String = "complete" }
data object BusinessCommandEvent5_discard : BusinessCommandEvent5 { override val `kind`: String = "discard" }
