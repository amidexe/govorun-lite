package com.govorun.lite.ui

import com.govorun.lite.account.SubscriptionPresentation
import com.govorun.lite.account.PromotionAccessKind
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.BillingRepository
import com.govorun.lite.account.checkoutConfirmationMessage
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.StarterActivationHandoff
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.home.HomeSnapshotReader
class SubscriptionActivity : ComponentActivity() {
    private val demoMode get() = BuildConfig.BILLING_DEMO_ENABLED
    private var presentation by mutableStateOf(com.govorun.lite.account.SubscriptionPresentation())
    private val checkoutRecovery by lazy { SubscriptionCheckoutRecovery(this) }
    private val billingPort by lazy { AndroidSubscriptionBilling(this) }
    private val controller by lazy {
        SubscriptionWindowAdapter(
            GovorunAccountRepository.billingBoundary,
            billingPort,
            AndroidSubscriptionEffects(this, checkoutRecovery),
            changed = { presentation = it },
        )
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        observeSubscriptionPolicy(
            onSessionChanged = controller::sessionChanged,
        )
        checkoutRecovery.observe()
        setContent {
            GovorunTheme {
                val billing by BillingRepository.state.collectAsStateWithLifecycle()
                val account by GovorunAccountRepository.state.collectAsStateWithLifecycle()
                val checkout by BillingRepository.window.collectAsStateWithLifecycle()
                val promotions by BillingRepository.promotions.collectAsStateWithLifecycle()
                val actions by billingPort.managementActions.collectAsStateWithLifecycle()
                val preview = checkout.preview?.takeIf { value ->
                    account.networkAvailable && account.profileFresh && billing.catalog.plans.any { it.offerId == value.offerId }
                }
                val previewId = checkout.previewId
                if (preview != null && previewId != null) {
                    val plan = billing.catalog.plans.firstOrNull { it.offerId == preview.offerId }
                    if (plan != null) {
                        SubscriptionCheckoutConfirmationScreen(
                            state = SubscriptionCheckoutConfirmation(
                                plan = plan,
                                purchaseKind = preview.purchaseKind,
                                trialDays = preview.trialDays,
                                freeAccessSource = preview.freeAccessSource,
                                freeAccessEndsAt = preview.freeAccessEndsAt,
                                firstChargeAt = preview.firstChargeAt,
                                receiptContactState = preview.receiptContactState,
                                receiptContactRequired = preview.clientCheckoutPreview?.receiptContactRequired == true,
                                receiptEmailMasked = preview.receiptEmailMasked,
                                receiptEmail = preview.receiptEmail,
                                amountDueNowKopeks = preview.amountDueNowKopeks,
                                firstChargeAmountKopeks = preview.firstChargeAmountKopeks,
                                renewalChargeAt = preview.renewalChargeAt,
                                renewalAmountKopeks = preview.renewalAmountKopeks,
                                servicePeriodEndsAt = preview.servicePeriodEndsAt,
                                subscriptionConsentToken = preview.subscriptionConsentToken,
                                legalDocuments = preview.legalDocuments,
                                firstChargeDelaySeconds = preview.firstChargeDelaySeconds,
                                paymentMethodKind = preview.paymentMethodKind,
                                availablePaymentMethods = preview.availablePaymentMethods,
                            ),
                            working = billing.working,
                            paymentAvailable = preview.clientCheckoutPreview?.canConfirm == true,
                            message = checkout.message,
                            onBack = { launchSubscription { controller.dismissCheckout() } },
                            onOpenLegalDocument = billingLegalDocumentHandler(this@SubscriptionActivity),
                            onPaymentMethod = { kind -> launchSubscription { controller.selectPaymentSource(previewId, kind) } },
                            onContinue = { receiptEmail ->
                                launchSubscription { controller.createCheckout(previewId, receiptEmail) }
                            },
                        )
                    }
                } else {
                    SubscriptionScreen(
                        state = billing,
                        checkoutUnavailable = checkout.checkoutUnavailable,
                        managementActions = actions,
                        account = account,
                        coreSetupReady = HomeSnapshotReader(this@SubscriptionActivity)
                            .read(justFinished = false).ready,
                        billingUiEnabled = billingClientSupportsCatalog(billing.catalog, demoMode),
                        demoMode = demoMode,
                        promoWorking = promotions.busy || checkout.busy,
                        starterWorking = presentation.starterWorking,
                        promoMessage = presentation.promoMessage ?: promotions.message,
                        canRetryPromoAccess = promotions.continuation?.access?.kind == com.govorun.lite.account.PromotionAccessKind.REFRESH,
                        onRetryPromoAccess = { launchSubscription { controller.retryPromoAccess() } },
                        promoPreview = promotions.preview,
                        onBack = ::cancelPromoAndFinish,
                        onCompleteSetup = { openCoreSetup(this@SubscriptionActivity) },
                        onSignIn = { openSubscriptionSignIn(this@SubscriptionActivity) },
                        onActivatePromo = { code, offer -> launchSubscription { controller.previewPromo(code, offer) } },
                        onConfirmPromo = { id -> launchSubscription { controller.confirmPromo(id) } },
                        onDismissPromoPreview = { launchSubscription { controller.dismissPromo() } },
                        onActivateStarter = { launchSubscription { controller.activateStarter() } },
                        onChoose = { plan -> check(!demoMode); launchSubscription { controller.choosePlan(plan) } },
                        onPreviewCheckout = if (billing.catalog.tBankPreviewEnabled) { plan ->
                            launchSubscription { controller.previewCheckout(plan) }
                        } else null,
                        onRestartCheckout = checkoutRecovery::restartBinding,
                        onResumeCheckout = checkoutRecovery::resumeCheckout,
                        onRetryBilling = { checkoutRecovery.refresh(full = true) },
                        onDemoPurchase = BillingRepository::simulatePurchase,
                        onDismissCheckoutForm = { launchSubscription { BillingRepository.dismissCheckoutForm() } },
                        onCancelRenewal = { cancelSubscriptionRenewal(demoMode, lifecycleScope) },
                        onRemovePaymentMethod = {
                            launchSubscription { BillingRepository.removePaymentMethod() }
                        },
                        onCancelPlanChange = { cancelScheduledSubscriptionChange(lifecycleScope) },
                        onContactSupport = ::openSubscriptionSupport,
                    )
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (acceptsTBankReturn(intent)) checkoutRecovery.refresh()
    }

    override fun onResume() {
        super.onResume()
        GovorunAccountRepository.refreshStyleCreditBalance()
        launchSubscription { controller.resumeStarter() }
        checkoutRecovery.refresh(full = true)
    }

    private fun cancelPromoAndFinish() {
        StarterActivationHandoff.clear(this)
        finish()
    }
}
