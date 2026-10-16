// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

sealed interface BusinessCommandEvent { val `kind`: String }
data class BusinessCommandEvent_start(
    val `force`: Boolean
) : BusinessCommandEvent { override val `kind`: String = "start" }
data class BusinessCommandEvent_read(
    val `configured`: Boolean,
    val `document`: BusinessCommandDocument,
    val `etag`: String,
    val `not_modified`: Boolean
) : BusinessCommandEvent { override val `kind`: String = "read" }
data class BusinessCommandEvent_written(
    val `document`: BusinessCommandDocument,
    val `etag`: String
) : BusinessCommandEvent { override val `kind`: String = "written" }
data class BusinessCommandEvent_failed(
    val `status`: Int?
) : BusinessCommandEvent { override val `kind`: String = "failed" }
