package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal fun subscriptionWindowDecision(action: String, access: SubscriptionAccess,
    accepted: Boolean = false, handoffPending: Boolean = false): SubscriptionWindowDecision {
    val result = SharedBusiness.evaluate(JSONObject().put("operation", "subscriptionInteraction")
        .put("action", action).put("accepted", accepted).put("handoffPending", handoffPending)
        .put("access", JSONObject().put("signedIn", access.signedIn)
            .put("hasEntitlement", access.hasEntitlement).put("starterEligible", access.starterEligible)
            .put("participated", access.promoProgram.participated)))
    return SubscriptionWindowDecision(result.getString("kind"),
        result.optString("message").takeUnless { result.isNull("message") })
}
