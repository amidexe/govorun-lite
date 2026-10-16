// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandIntent { val `kind`: String }
data class BusinessCommandIntent_change(
    val `changes`: BusinessCommandInputChanges,
    val `local`: BusinessCommandValues
) : BusinessCommandIntent { override val `kind`: String = "change" }
data class BusinessCommandIntent_replace(
    val `local`: BusinessCommandValues
) : BusinessCommandIntent { override val `kind`: String = "replace" }
data class BusinessCommandIntent_compare(
    val `after`: BusinessCommandValues,
    val `before`: BusinessCommandValues
) : BusinessCommandIntent { override val `kind`: String = "compare" }
