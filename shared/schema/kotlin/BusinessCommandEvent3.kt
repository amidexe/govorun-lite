// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent3 { val `kind`: String }
data object BusinessCommandEvent3_start : BusinessCommandEvent3 { override val `kind`: String = "start" }
data class BusinessCommandEvent3_document(
    val `document`: BusinessCommandDocument2
) : BusinessCommandEvent3 { override val `kind`: String = "document" }
data class BusinessCommandEvent3_failed(
    val `status`: Int?
) : BusinessCommandEvent3 { override val `kind`: String = "failed" }
