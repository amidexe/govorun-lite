// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandStyleWindowFacts(
    val `accessActive`: Boolean,
    val `accessType`: String,
    val `balanceKind`: String,
    val `dailyLimit`: Long,
    val `dailyResetPassed`: Boolean,
    val `entitlementActive`: Boolean,
    val `freeAccessAvailable`: Boolean,
    val `issuanceInFlight`: Boolean,
    val `localBound`: Boolean,
    val `localKnown`: Boolean,
    val `pendingCredits`: Long,
    val `pendingUsageToday`: Long,
    val `pendingUsageTotal`: Long,
    val `presentationAccess`: Any?,
    val `processingReady`: Boolean,
    val `profileFresh`: Boolean,
    val `readyCredits`: Long,
    val `recoveryInFlight`: Boolean,
    val `remainingToday`: Long,
    val `remainingTotal`: Long,
    val `reserveReturn`: Boolean,
    val `signedIn`: Boolean,
    val `starterAllocated`: Boolean,
    val `totalLimit`: Long,
    val `usageAccounting`: Boolean,
    val `usageFresh`: Boolean,
    val `usageLedger`: BusinessCommandLedger?,
    val `usageScope`: BusinessCommandScope3?
)
