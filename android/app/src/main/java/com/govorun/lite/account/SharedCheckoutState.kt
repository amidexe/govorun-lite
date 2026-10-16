package com.govorun.lite.account

import org.json.JSONObject

/** Projects the common checkout owner’s admission and current server session. */
internal class SharedCheckoutState {
    private var appliedRevision = -1L

    fun accept(operation: BillingOperation, snapshot: JSONObject) = operation.update { previous ->
        require(snapshot.getLong("epoch") == operation.scope.generation)
        val revision = snapshot.getLong("revision")
        if (revision <= appliedRevision) previous else {
            val pending = snapshot.get("pending") as Boolean
            val purchasePending = snapshot.get("purchasePending") as Boolean
            val checking = snapshot.get("checking") as Boolean
            val sessionId = snapshot.optString("sessionId").takeUnless { snapshot.isNull("sessionId") || it.isEmpty() }
            appliedRevision = revision
            previous.copy(checkoutPending = pending, checkoutPurchasePending = purchasePending,
                checkoutChecking = checking, checkoutSessionId = sessionId,
                checkoutSessionGeneration = operation.scope.generation)
        }
    }
}
