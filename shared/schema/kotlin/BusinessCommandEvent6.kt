// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent6 { val `kind`: String }
data class BusinessCommandEvent6_begin(
    val `bypass`: Boolean,
    val `streaming`: Boolean
) : BusinessCommandEvent6 { override val `kind`: String = "begin" }
data class BusinessCommandEvent6_draft(
    val `active`: Boolean
) : BusinessCommandEvent6 { override val `kind`: String = "draft" }
data class BusinessCommandEvent6_route(
    val `enabled`: Boolean,
    val `self_hosted`: Boolean
) : BusinessCommandEvent6 { override val `kind`: String = "route" }
data class BusinessCommandEvent6_official(
    val `accepted_terms`: Int,
    val `access`: BusinessCommandAccess,
    val `current_terms`: Int,
    val `enabled`: Boolean,
    val `units`: List<Int>
) : BusinessCommandEvent6 { override val `kind`: String = "official" }
data class BusinessCommandEvent6_configuration(
    val `configured`: Boolean
) : BusinessCommandEvent6 { override val `kind`: String = "configuration" }
data class BusinessCommandEvent6_selection(
    val `enabled`: Boolean
) : BusinessCommandEvent6 { override val `kind`: String = "selection" }
data class BusinessCommandEvent6_prepared(
    val `access`: BusinessCommandAccess
) : BusinessCommandEvent6 { override val `kind`: String = "prepared" }
data class BusinessCommandEvent6_network(
    val `available`: Boolean?
) : BusinessCommandEvent6 { override val `kind`: String = "network" }
data class BusinessCommandEvent6_response(
    val `input`: List<Int>,
    val `output`: List<Int>
) : BusinessCommandEvent6 { override val `kind`: String = "response" }
data class BusinessCommandEvent6_failure(
    val `reason`: String
) : BusinessCommandEvent6 { override val `kind`: String = "failure" }
