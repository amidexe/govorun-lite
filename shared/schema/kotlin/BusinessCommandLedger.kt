// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandLedger(
    val `acknowledged`: Map<String, Map<String, Int>>,
    val `pending`: Map<String, BusinessCommandReservation>,
    val `requests`: Map<String, Map<String, BusinessCommandRequestCount>>,
    val `totals`: Map<String, Map<String, Int>>
)
