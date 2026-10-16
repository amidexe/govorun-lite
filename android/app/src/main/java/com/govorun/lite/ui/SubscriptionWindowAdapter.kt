package com.govorun.lite.ui

import com.govorun.lite.account.SubscriptionBilling
import com.govorun.lite.account.SubscriptionEffects
import com.govorun.lite.account.SubscriptionPresentation
import com.govorun.lite.account.SubscriptionFailure
import com.govorun.lite.account.PromotionAccessKind
import com.govorun.lite.account.PromotionAccess
import com.govorun.lite.account.PaymentLaunch
import com.govorun.lite.account.SubscriptionCommand
import com.govorun.lite.account.SubscriptionEffect

import com.govorun.lite.model.billing.*

import com.govorun.lite.account.SessionBoundary
import java.util.concurrent.CancellationException

/** Renders common decisions and executes dialogs/browser/SDK effects under a native session lease. */
internal class SubscriptionWindowAdapter(
    private val sessions: SessionBoundary<BillingIdentity>,
    private val billing: SubscriptionBilling,
    private val effects: SubscriptionEffects,
    private val changed: (SubscriptionPresentation) -> Unit,
) {
    var state = SubscriptionPresentation()
        private set
    private var identity = sessions.capture().identity

    fun sessionChanged() {
        val lease = sessions.capture()
        lease.publish { synchronizeIdentity(lease.identity) }
    }

    private fun synchronizeIdentity(current: BillingIdentity) {
        if (identity == current) return
        identity = current
        update { SubscriptionPresentation() }
    }

    suspend fun dismissPromo() = action { lease -> checkout(lease, SubscriptionCommand.DismissPromotion) }
    suspend fun dismissCheckout() = action { lease -> lease.request { billing.dismissCheckoutForm() } }

    suspend fun previewPromo(code: String, offerId: String) = action { lease ->
        if (state.consentPromptOpen) return@action
        if (billing.interaction("authenticate").kind == "signIn") { lease.publish(effects::signIn); return@action }
        lease.publish { update { it.copy(promoMessage = null) } }
        checkout(lease, SubscriptionCommand.PreviewPromotion(code, offerId))
    }

    suspend fun confirmPromo(previewId: String) = action { lease ->
        checkout(lease, SubscriptionCommand.ConfirmPromotion(previewId))
    }

    suspend fun retryPromoAccess() = action { lease ->
        val id = billing.promotions.continuation?.id ?: return@action
        checkout(lease, SubscriptionCommand.RefreshPromotionAccess(id))
    }

    suspend fun activateStarter() = action { lease ->
        if (state.starterWorking || billing.promotions.busy || state.consentPromptOpen) return@action
        if (billing.interaction("startFreeAccess").kind == "rememberAndSignIn") {
            lease.publish { effects.beginStarterHandoff(); effects.signIn() }
        } else continueStarter(lease)
    }

    suspend fun resumeStarter() = action { lease ->
        if (state.starterWorking || state.consentPromptOpen || !effects.starterHandoffPending()) return@action
        val decision = billing.interaction("resumeFreeAccess", handoffPending = true)
        when (decision.kind) {
            "activate" -> continueStarter(lease)
            "clearHandoff", "clearAndRefresh" -> {
                lease.publish {
                    effects.clearStarterHandoff()
                    update { it.copy(promoMessage = decision.message) }
                }
                if (decision.kind == "clearAndRefresh") lease.request { billing.load() }
            }
        }
    }

    suspend fun previewCheckout(plan: BillingPlan) = action { lease ->
        if (billing.billing.working) return@action
        if (billing.interaction("authenticate").kind == "signIn") { lease.publish(effects::signIn); return@action }
        checkout(lease, SubscriptionCommand.Preview(plan.offerId))
    }

    suspend fun selectPaymentSource(previewId: String, kind: String) = action { lease ->
        checkout(lease, SubscriptionCommand.ChooseMethod(previewId, kind))
    }

    suspend fun createCheckout(previewId: String, email: String?) = action { lease ->
        checkout(lease, SubscriptionCommand.Confirm(previewId, email))
    }

    private suspend fun checkout(lease: SessionBoundary<BillingIdentity>.Lease, command: SubscriptionCommand) {
        try {
            when (val effect = lease.request { billing.checkout(command) }) {
                null -> Unit
                is SubscriptionEffect.PromotionAccess -> {
                    if (effect.value.kind == PromotionAccessKind.ACCEPT_TERMS) {
                        val id = requireNotNull(effect.continuationId)
                        lease.publish { update { it.copy(consentPromptOpen = true) } }
                        try {
                            val accepted = lease.request(effects::requestStyleConsent)
                            lease.request { billing.checkout(SubscriptionCommand.DismissPromotionAccess(id, !accepted)) }
                        } finally { lease.publish { update { it.copy(consentPromptOpen = false) } } }
                    }
                }
                is SubscriptionEffect.OpenUrl -> lease.publish {
                    when (effects.openPayment(effect.url)) {
                        PaymentLaunch.OPENED -> Unit
                        PaymentLaunch.FAILED -> billing.purchaseFailed()
                        PaymentLaunch.TEST_BROWSER_MISSING -> {
                            billing.purchaseFailed()
                            update { it.copy(promoMessage = "Для тестовой оплаты нужен Firefox Nightly с настроенным тестовым подключением.") }
                        }
                    }
                }
            }
        } catch (error: CancellationException) { throw error }
        catch (_: Exception) { /* The common window projection owns the failure and disclosure. */ }
    }

    suspend fun choosePlan(plan: BillingPlan) = action { lease ->
        if (billing.billing.working) return@action
        when (billing.interaction("choosePlan").kind) {
            "signIn" -> lease.publish(effects::signIn)
            "previewBank" -> previewCheckout(plan)
        }
    }

    private suspend fun continueStarter(lease: SessionBoundary<BillingIdentity>.Lease) {
        val program = billing.access.promoProgram
        lease.publish { update { it.copy(starterWorking = true, consentPromptOpen = true, promoMessage = null) } }
        try {
            val activated = lease.request { effects.activateStarterWithConsent(program) }
            lease.publish {
                effects.clearStarterHandoff()
                if (activated) effects.dismissStarterGuidance()
                update { it.copy(promoMessage = billing.interaction("freeAccessCompleted", accepted = activated).message) }
            }
            if (activated) lease.request { billing.load() }
        } finally {
            lease.publish { update { it.copy(starterWorking = false, consentPromptOpen = false) } }
        }
    }

    private suspend fun action(block: suspend (SessionBoundary<BillingIdentity>.Lease) -> Unit) {
        val lease = sessions.capture()
        lease.publish { synchronizeIdentity(lease.identity) }
        lease.request { block(lease) }
    }

    private fun fail(lease: SessionBoundary<BillingIdentity>.Lease, error: Exception) {
        if (error is CancellationException) throw error
        lease.publish { update { it.copy(promoMessage = (error as? SubscriptionFailure)?.notice
            ?: "Не удалось выполнить действие. Попробуйте ещё раз.") } }
    }

    private fun update(change: (SubscriptionPresentation) -> SubscriptionPresentation) {
        state = change(state)
        changed(state)
    }
}
