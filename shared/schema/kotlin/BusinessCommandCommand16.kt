// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand16 { val `kind`: String }
data class BusinessCommandCommand16_begin(
    val `input`: List<Int>
) : BusinessCommandCommand16 { override val `kind`: String = "begin" }
data class BusinessCommandCommand16_response(
    val `input`: List<Int>,
    val `output`: List<Int>
) : BusinessCommandCommand16 { override val `kind`: String = "response" }
data class BusinessCommandCommand16_failure(
    val `status`: Int
) : BusinessCommandCommand16 { override val `kind`: String = "failure" }
