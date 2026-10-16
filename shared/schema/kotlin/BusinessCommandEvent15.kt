// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent15 { val `kind`: String }
data class BusinessCommandEvent15_begin(
    val `accept_terms`: Boolean,
    val `confirmed`: Boolean,
    val `operation_id`: String,
    val `selection`: String,
    val `selection_exists`: Boolean,
    val `terms_version`: Int
) : BusinessCommandEvent15 { override val `kind`: String = "begin" }
data class BusinessCommandEvent15_complete(
    val `intent`: BusinessCommandIntent2,
    val `lifetime`: BusinessCommandLifetime,
    val `selection_exists`: Boolean
) : BusinessCommandEvent15 { override val `kind`: String = "complete" }
data class BusinessCommandEvent15_failed(
    val `generation`: Long,
    val `lifetime`: BusinessCommandLifetime,
    val `operation_id`: String
) : BusinessCommandEvent15 { override val `kind`: String = "failed" }
