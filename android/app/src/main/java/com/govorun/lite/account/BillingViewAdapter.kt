package com.govorun.lite.account

import com.govorun.lite.ui.BillingDemoScenario
import com.govorun.lite.BuildConfig
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONObject

/** UI composition for common subscription, promotion and native store ports. */
internal class BillingStore(private val host: BillingEnvironment) : BillingService {
    private val mutableState = MutableStateFlow(BillingUiState())
    private val mutableWindow = MutableStateFlow(SubscriptionWindow())
    private val billing = SharedBillingSession(host)
    private val mutablePromotions = MutableStateFlow(PromotionWindow())
    private var actionError: String? = null
    private var actionErrorEpoch: Long? = null
    override val promotions: StateFlow<PromotionWindow> = mutablePromotions.asStateFlow()
    private val operations = BillingOperations(host.boundary,
        { mutableState.value }, { mutableState.value = it })
    override val state: StateFlow<BillingUiState> = mutableState.asStateFlow()
    override val window: StateFlow<SubscriptionWindow> = mutableWindow.asStateFlow()

    override suspend fun load() {
        try { subscription(SubscriptionCommand.Refresh) }
        catch (error: CancellationException) { throw error }
        catch (_: Exception) { /* The common projection or transport adapter displays the failure. */ }
    }

    override fun acceptAccountProjection(value: JSONObject) {
        val lease = host.boundary.capture()
        if (value.getJSONObject("snapshot").getLong("sessionGeneration") != lease.identity.generation) return
        lease.publish {
            applyProjection(BillingOperation(lease, { mutableState.value }, { mutableState.value = it }), value)
        }
    }

    private fun applyProjection(operation: BillingOperation, value: JSONObject) {
        billing.acceptSnapshot(operation, value.getJSONObject("billingStatus"))
        billing.acceptCheckoutSnapshot(operation, value.getJSONObject("checkout"))
        val next = value.subscriptionWindow(operation.scope.generation)
        val changed = next != mutableWindow.value
        operation.update {
            val projected = it.withSubscriptionProjection(value, next, changed)
            actionError?.takeIf { actionErrorEpoch == operation.scope.generation }
                ?.let { message -> projected.copy(message = message, checkoutMessage = message, messageIsPrimary = true) }
                ?: projected
        }
        val promotion = value.promotionWindow(operation.scope.generation)
        operation.storage { mutableWindow.value = next; mutablePromotions.value = promotion }
    }

    override suspend fun subscription(command: SubscriptionCommand): SubscriptionEffect? = operations.run(mutating = false) {
        val wire = command.wire(ui.catalog)
        val userInitiated = wire.getBoolean("userInitiated")
        if (userInitiated) {
            storage { actionError = null; actionErrorEpoch = scope.generation }
            update { it.copy(message = null, checkoutMessage = null) }
        }
        try {
            val result = host.command(this, JSONObject().put("operation", "subscriptionAction")
                .put("command", wire))
            applyProjection(this, host.accountProjection())
            if (result.optBoolean("failed") && userInitiated) throw SubscriptionFailure(requireNotNull(mutableWindow.value.message))
            val effect = result.optJSONObject("effect") ?: return@run null
            when (effect.getString("kind")) {
                "openUrl" -> SubscriptionEffect.OpenUrl(effect.getString("url"))
                "promotionAccess" -> SubscriptionEffect.PromotionAccess(effect.getJSONObject("access").toPromotionAccess(),
                    effect.get("continuationId").takeUnless { it == JSONObject.NULL } as String?)
                else -> error("Unsupported common subscription effect")
            }
        } catch (error: CancellationException) { throw error }
        catch (error: Exception) {
            if (userInitiated && error !is SubscriptionFailure) {
                storage {
                    actionError = "Не удалось проверить состояние подписки. Попробуйте ещё раз."
                    actionErrorEpoch = scope.generation
                }
                update { it.copy(message = actionError, checkoutMessage = actionError, messageIsPrimary = true) }
            }
            throw error
        } finally {
            if (isCurrent()) applyProjection(this, host.accountProjection())
        }
    }

    override fun simulatePurchase(planCode: String) {
        check(BuildConfig.BILLING_DEMO_ENABLED)
        val current = mutableState.value.subscription
        val next = BillingDemoScenario.purchase(current, planCode)
        val message = BillingDemoScenario.purchaseMessage(current, planCode)
        mutableState.value = mutableState.value.copy(subscription = next, message = message)
    }

    override fun simulateCancellation() {
        check(BuildConfig.BILLING_DEMO_ENABLED)
        mutableState.value = mutableState.value.copy(
            subscription = BillingDemoScenario.cancel(mutableState.value.subscription),
            message = "Автопродление тестовой подписки отключено",
        )
    }

    override suspend fun cancelScheduledPlanChange() { subscription(SubscriptionCommand.Manage("cancelPlanChange")) }
    override suspend fun cancelRenewal() { subscription(SubscriptionCommand.Manage("cancelRenewal")) }
    override suspend fun removePaymentMethod() { subscription(SubscriptionCommand.Manage("removeCard")) }
    override suspend fun requestRefund(reasonCode: String) { subscription(SubscriptionCommand.Manage("requestRefund", reasonCode)) }
    override suspend fun dismissCheckoutForm() { subscription(SubscriptionCommand.Dismiss) }

    override fun purchaseFailed() = setWorking(false, com.govorun.lite.shared.SharedBusiness.evaluate(
        JSONObject().put("operation", "billingLaunchFailure")).getString("message"), true)

    override fun clear() {
        actionError = null; actionErrorEpoch = null
        mutableWindow.value = SubscriptionWindow()
        mutablePromotions.value = PromotionWindow()
        mutableState.value = BillingUiState(catalog = mutableState.value.catalog)
    }

    private fun setWorking(working: Boolean, message: String? = null, messageIsPrimary: Boolean = false) {
        mutableState.value = mutableState.value.copy(working = working, message = message,
            checkoutMessage = message, messageIsPrimary = messageIsPrimary, checkoutFormFailure = null)
    }
}
