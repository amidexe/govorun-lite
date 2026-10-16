// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandCommand6(
    val `event`: BusinessCommandEvent,
    val `flow`: BusinessCommandFlow?,
    val `local`: BusinessCommandDocument,
    val `nowMillis`: Long,
    val `state`: BusinessCommandState?
)
