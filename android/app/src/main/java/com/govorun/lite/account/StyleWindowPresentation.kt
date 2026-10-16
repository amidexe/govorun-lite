package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

/** Native accounting facts in; shared display/admission decisions out. */
internal data class StyleWindowPresentation(
    val remainingToday: Int,
    val remainingTotal: Int,
    val starterLocallyExhausted: Boolean,
    val starterLimitExhausted: Boolean,
    val setupNeedsSubscription: Boolean,
    val availability: String,
    val pauseReason: String,
    val freshUsageAvailable: Boolean,
    val prepareOnDemand: Boolean,
    val prepareInBackground: Boolean,
    val processingReadiness: StyleProcessingReadiness,
    val quota: StyleQuotaPresentation? = null,
)

internal fun AccountState.styleWindowFacts(): JSONObject {
    val usage = styleAccess.usageBalance
    return JSONObject().put("signedIn", signedIn)
        .put("profileFresh", profileFresh)
        .put("presentationAccess", styleAccess.quotaWire())
        .put("entitlementActive", authority.entitlementActive)
        .put("processingReady", processingReady).put("freeAccessAvailable", freeAccessAvailable)
        .put("accessType", styleAccess.type).put("accessActive", styleAccess.active)
        .put("balanceKind", styleAccess.balanceKind).put("starterAllocated", starterGrant.allocated)
        .put("dailyLimit", styleAccess.dailyLimit).put("totalLimit", styleAccess.totalLimit)
        .put("remainingToday", usage?.remainingToday ?: styleAccess.remainingToday)
        .put("remainingTotal", usage?.remainingTotal ?: styleAccess.remainingTotal)
        .put("usageAccounting", usage != null).put("usageFresh", false)
        .put("pendingUsageToday", pendingStyleUsageToday).put("pendingUsageTotal", pendingStyleUsageTotal)
        .put("localKnown", localStylePassStateKnown).put("localBound", localStylePassStateKnown)
        .put("readyCredits", localUnspentStyleCredits).put("pendingCredits", localPendingStyleCredits)
        .put("issuanceInFlight", localStylePassIssuancePending).put("recoveryInFlight", false)
        .put("dailyResetPassed", false)
        .put("reserveReturn", styleAccess.supportsReserveReturn)
}

internal fun AccountState.styleWindowPresentation(): StyleWindowPresentation {
    val result = styleWindowJson?.let(::JSONObject) ?: SharedBusiness.evaluate(
        JSONObject().put("operation", "styleWindowPresentation").put("facts", styleWindowFacts()))
    return StyleWindowPresentation(result.getInt("remainingToday"), result.getInt("remainingTotal"),
        result.getBoolean("starterLocallyExhausted"), result.getBoolean("starterLimitExhausted"),
        result.getBoolean("setupNeedsSubscription"), result.getString("availability"), result.getString("pauseReason"),
        result.getBoolean("freshUsageAvailable"), result.getBoolean("prepareOnDemand"), result.getBoolean("prepareInBackground"),
        StyleProcessingReadiness.read(result.getString("processingReadiness")),
        result.optJSONObject("quota")?.let(::StyleQuotaPresentation))
}

internal enum class StyleProcessingReadiness {
    READY, ON_DEMAND, UNAVAILABLE, PREPARING, NEEDS_PREPARATION;

    companion object {
        fun read(value: String): StyleProcessingReadiness = when (value) {
            "ready" -> READY
            "onDemand" -> ON_DEMAND
            "unavailable" -> UNAVAILABLE
            "preparing" -> PREPARING
            "needsPreparation" -> NEEDS_PREPARATION
            else -> error("Unknown common processing readiness")
        }
    }
}
