// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandAccessInteractionFacts(
    val `authorized`: Boolean,
    val `consentAccepted`: Boolean,
    val `enableAllowed`: Boolean,
    val `entitlementActive`: Boolean,
    val `freeAccessAvailable`: Boolean,
    val `signedIn`: Boolean,
    val `storageAvailable`: Boolean,
    val `termsActionAvailable`: Boolean
)
