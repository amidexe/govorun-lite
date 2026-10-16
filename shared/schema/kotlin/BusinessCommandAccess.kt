// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandAccess(
    val `configured`: Boolean,
    val `entitlementActive`: Boolean,
    val `networkAvailable`: Boolean?,
    val `processingReady`: Boolean,
    val `selected`: Boolean,
    val `serviceAvailable`: Boolean,
    val `sessionCurrent`: Boolean,
    val `storageAvailable`: Boolean,
    val `termsRequired`: Boolean
)
