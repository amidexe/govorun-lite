// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand { val `kind`: String }
data class BusinessCommandCommand_controls(
    val `facts`: Any?
) : BusinessCommandCommand { override val `kind`: String = "controls" }
data class BusinessCommandCommand_facts(
    val `access`: Any?
) : BusinessCommandCommand { override val `kind`: String = "facts" }
data class BusinessCommandCommand_period(
    val `first`: Any?,
    val `second`: Any?
) : BusinessCommandCommand { override val `kind`: String = "period" }
data class BusinessCommandCommand_match(
    val `current`: BusinessCommandScope,
    val `scope`: BusinessCommandScope
) : BusinessCommandCommand { override val `kind`: String = "match" }
data class BusinessCommandCommand_prewire(
    val `same_scope`: Boolean
) : BusinessCommandCommand { override val `kind`: String = "prewire" }
data class BusinessCommandCommand_refreshFailure(
    val `status`: Int?
) : BusinessCommandCommand { override val `kind`: String = "refreshFailure" }
data class BusinessCommandCommand_enabled(
    val `consent`: Boolean,
    val `enabled`: Boolean,
    val `processing_ready`: Boolean,
    val `protocol_supported`: Boolean,
    val `self_hosted`: Boolean
) : BusinessCommandCommand { override val `kind`: String = "enabled" }
