// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandCommand10(
    val `event`: BusinessCommandEvent3,
    val `flow`: BusinessCommandFlow3?,
    val `intents`: List<BusinessCommandIntent>,
    val `local`: BusinessCommandValues,
    val `state`: BusinessCommandState3?
)
