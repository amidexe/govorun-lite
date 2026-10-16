// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BusinessCommandContext2(
    val `backgroundAllowed`: Boolean,
    val `explicitDemand`: Boolean,
    val `issuanceCountersObserved`: Boolean,
    val `issuanceRemainingToday`: Long,
    val `issuanceRemainingTotal`: Long,
    val `pendingIssuanceCredits`: Long,
    val `replacementReceipts`: Long,
    val `reserveCredits`: Long,
    val `retiringCredits`: Long
)
