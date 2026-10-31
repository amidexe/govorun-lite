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
    /** The own limits of Govorun's products besides dictation (Говорун.Эхо, later others). */
    val products: List<ProductWindow> = emptyList(),
)

/** A product's own limit today and in the period, with this phone's requests the account card does not count yet. */
internal data class ProductWindow(val product: String, val limitToday: Int, val remainingToday: Int, val limitTotal: Int = 0,
                                  val remainingTotal: Int = 0) {
    val usedToday: Int get() = limitToday - remainingToday.coerceIn(0, limitToday)
    val percentLeft: Int get() = percent(remainingToday, limitToday)!!
    val periodPercentLeft: Int? get() = percent(remainingTotal, limitTotal)

    private fun percent(remaining: Int, limit: Int): Int? =
        if (limit <= 0) null else (remaining.coerceIn(0, limit) * 100 + limit / 2) / limit
}

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
        result.optJSONObject("quota")?.let(::StyleQuotaPresentation),
        result.optJSONArray("products")?.let { lines -> (0 until lines.length()).map(lines::getJSONObject).map {
            ProductWindow(it.getString("product"), it.getInt("limitToday"), it.getInt("remainingToday"), it.optInt("limitTotal"),
                it.optInt("remainingTotal"))
        } }.orEmpty().filter { it.limitToday > 0 })
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
