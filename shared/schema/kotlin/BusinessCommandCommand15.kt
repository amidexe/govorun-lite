// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand15 { val `kind`: String }
data class BusinessCommandCommand15_proof(
    val `body`: String,
    val `date`: String?,
    val `modified`: String?,
    val `status`: Int
) : BusinessCommandCommand15 { override val `kind`: String = "proof" }
