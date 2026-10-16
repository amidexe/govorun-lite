// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent9 { val `kind`: String }
data class BusinessCommandEvent9_starting(
    val `id`: String
) : BusinessCommandEvent9 { override val `kind`: String = "starting" }
data class BusinessCommandEvent9_accepted(
    val `id`: String
) : BusinessCommandEvent9 { override val `kind`: String = "accepted" }
data class BusinessCommandEvent9_ambiguous(
    val `id`: String
) : BusinessCommandEvent9 { override val `kind`: String = "ambiguous" }
data class BusinessCommandEvent9_defer(
    val `id`: String,
    val `milliseconds`: Long
) : BusinessCommandEvent9 { override val `kind`: String = "defer" }
data class BusinessCommandEvent9_forget(
    val `id`: String
) : BusinessCommandEvent9 { override val `kind`: String = "forget" }
data class BusinessCommandEvent9_inspect(
    val `expected_version`: Long?,
    val `id`: String,
    val `observe`: Boolean
) : BusinessCommandEvent9 { override val `kind`: String = "inspect" }
data class BusinessCommandEvent9_select(
    val `ids`: List<String>
) : BusinessCommandEvent9 { override val `kind`: String = "select" }
data class BusinessCommandEvent9_scope(
    val `epoch`: Long,
    val `ids`: List<String>
) : BusinessCommandEvent9 { override val `kind`: String = "scope" }
data class BusinessCommandEvent9_current(
    val `id`: String,
    val `version`: Long
) : BusinessCommandEvent9 { override val `kind`: String = "current" }
data class BusinessCommandEvent9_claim(
    val `id`: String,
    val `owner`: String,
    val `version`: Long
) : BusinessCommandEvent9 { override val `kind`: String = "claim" }
data class BusinessCommandEvent9_release(
    val `id`: String,
    val `owner`: String,
    val `version`: Long
) : BusinessCommandEvent9 { override val `kind`: String = "release" }
