// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandReservation(
    val `creditsByScope`: Map<String, Int>,
    val `day`: String,
    val `requestCredits`: Int?
)
