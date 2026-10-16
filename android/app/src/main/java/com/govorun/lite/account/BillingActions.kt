package com.govorun.lite.account

import org.json.JSONObject

/** Shared presentation decisions. Native subscription fields cannot enable an action. */
internal data class BillingActions(
    val cancelRenewal: Boolean = false,
    val cancelPlanChange: Boolean = false,
    val removeCard: Boolean = false,
    val requestRefund: Boolean = false,
) {
    companion object {
        val None = BillingActions()
        fun read(projection: JSONObject, generation: Long): BillingActions {
            val value = projection.optJSONObject("billingActions") ?: return None
            val epoch = when (val raw = value.opt("epoch")) {
                is Int -> raw.toLong()
                is Long -> raw
                else -> return None
            }
            if (epoch != generation) return None
            return BillingActions(value.opt("cancelRenewal") == true, value.opt("cancelPlanChange") == true,
                value.opt("removeCard") == true, value.opt("requestRefund") == true)
        }
    }
}
