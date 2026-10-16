// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandCommand13 { val `kind`: String }
data class BusinessCommandCommand13_readResponse(
    val `body`: String,
    val `etag`: String?,
    val `fallback`: String,
    val `status`: Int
) : BusinessCommandCommand13 { override val `kind`: String = "readResponse" }
data class BusinessCommandCommand13_validate(
    val `document`: BusinessCommandDocument
) : BusinessCommandCommand13 { override val `kind`: String = "validate" }
data class BusinessCommandCommand13_merge(
    val `base`: BusinessCommandDocument,
    val `local`: BusinessCommandDocument,
    val `remote`: BusinessCommandDocument
) : BusinessCommandCommand13 { override val `kind`: String = "merge" }
