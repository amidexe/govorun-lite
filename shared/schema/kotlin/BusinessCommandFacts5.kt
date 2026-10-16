// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandFacts5(
    val `authorized`: Boolean,
    val `availability`: String,
    val `clientTermsVersion`: Int,
    val `consentAccepted`: Boolean,
    val `enabled`: Boolean,
    val `entitled`: Boolean,
    val `generation`: Long,
    val `localTermsVersion`: Int,
    val `paused`: Boolean,
    val `permissionsReady`: Boolean,
    val `presetCount`: Int,
    val `processingReadiness`: String,
    val `processingReady`: Boolean,
    val `profileFresh`: Boolean,
    val `selfHosted`: Boolean,
    val `selfHostedReady`: Boolean,
    val `signedIn`: Boolean,
    val `starterLocallyExhausted`: Boolean,
    val `storageAvailable`: Boolean,
    val `termsMatch`: Boolean
)
