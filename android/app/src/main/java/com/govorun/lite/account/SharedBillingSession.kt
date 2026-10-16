package com.govorun.lite.account

import org.json.JSONObject

/** Account transport and managed projection adapter for the common billing owner. */
internal class SharedBillingSession(private val host: BillingEnvironment) {
    private val checkout = SharedCheckoutState()
    fun acceptCheckoutSnapshot(operation: BillingOperation, snapshot: JSONObject) = checkout.accept(operation, snapshot)
    private var appliedRevision = -1L
    private data class Projection(val epoch: Long, val status: BillingStatus, val fresh: Boolean,
        val message: String?, val needsCheck: Boolean,
        val checkout: com.govorun.lite.model.billing.BillingCheckoutPresentation)
    @Volatile private var projection: Projection? = null

    fun currentStatus(operation: BillingOperation): BillingStatus? {
        operation.ensureCurrent()
        return projection?.takeIf { it.epoch == operation.scope.generation && it.fresh }?.status
    }

    fun publishCurrent(operation: BillingOperation) = operation.update { previous ->
        projection?.takeIf { it.epoch == operation.scope.generation }?.let { apply(previous, it) } ?: previous
    }

    private fun apply(previous: BillingUiState, value: Projection) = previous.copy(
        subscription = value.status.subscription, starterGrant = value.status.starterGrant,
        statusUnavailable = !value.fresh, message = value.message ?: previous.message,
        managementStatusPending = value.needsCheck,
        checkoutPresentation = value.checkout,
        checkoutMessage = value.message ?: previous.copy(checkoutPresentation = value.checkout).projectCheckoutMessage(),
    )

    fun acceptSnapshot(operation: BillingOperation, snapshot: JSONObject) = operation.update { previous ->
        val revision = snapshot.getLong("revision")
        if (revision <= appliedRevision) previous else {
            appliedRevision = revision
            val value = try {
                Projection(snapshot.getLong("epoch"), snapshot.optJSONObject("status")?.toBillingStatus() ?: BillingStatus(),
                    snapshot.getBoolean("fresh"), snapshot.optString("message").takeUnless { snapshot.isNull("message") },
                    snapshot.optJSONObject("management")?.optBoolean("needsCheck") == true,
                    snapshot.optJSONObject("checkout")?.toBillingCheckoutPresentation()
                        ?: com.govorun.lite.model.billing.BillingCheckoutPresentation())
            } catch (error: Exception) { projection = null; throw error }
            projection = value
            apply(previous, value)
        }
    }

    fun acceptCurrent(operation: BillingOperation) {
        operation.ensureCurrent()
        val value = host.accountProjection()
        require(value.getJSONObject("snapshot").getLong("sessionGeneration") == operation.scope.generation)
        acceptSnapshot(operation, value.getJSONObject("billingStatus"))
        acceptCheckoutSnapshot(operation, value.getJSONObject("checkout"))
    }

}
