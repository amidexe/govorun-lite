package com.govorun.lite.ui
import com.govorun.lite.model.billing.unavailable
import com.govorun.lite.model.billing.unavailableMessage
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import com.govorun.lite.stylization.hasInlineStylizationConsent
import com.govorun.lite.stylization.InlineStylizationConsentCard
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.BillingPlan
import com.govorun.lite.account.purchaseAvailable
import com.govorun.lite.account.BillingActions
import com.govorun.lite.account.presentation
import com.govorun.lite.account.subscriptionDetailsAvailable
import com.govorun.lite.account.subscriptionInformationUpdating
import com.govorun.lite.account.BillingUiState
import com.govorun.lite.account.CHECKOUT_UNAVAILABLE_MESSAGE
import com.govorun.lite.account.connectionMessage
import com.govorun.lite.account.StarterStyleLimitPresentation
import com.govorun.lite.account.activeStyleLimitPresentation
import com.govorun.lite.account.bonusStyleLimitPresentation
import com.govorun.lite.account.bonusAccessStatus
import com.govorun.lite.account.bonusAccessTitle
import com.govorun.lite.account.starterStyleLimitPresentation
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import com.product.ui.productScreenPadding

@Composable
internal fun SubscriptionScreen(
    state: BillingUiState,
    account: AccountState,
    managementActions: BillingActions = BillingActions.None,
    checkoutUnavailable: Boolean = false,
    coreSetupReady: Boolean = true,
    billingUiEnabled: Boolean,
    demoMode: Boolean,
    promoWorking: Boolean,
    starterWorking: Boolean,
    promoMessage: String?,
    canRetryPromoAccess: Boolean = false,
    onRetryPromoAccess: () -> Unit = {},
    promoPreview: com.govorun.lite.account.PromotionPreview?,
    initialTariffsExpanded: Boolean = false,
    initialSelectedPlanCode: String? = null,
    onPreviewCheckout: ((BillingPlan) -> Unit)? = null,
    onRetryBilling: () -> Unit = {},
    onRestartCheckout: () -> Unit = {},
    onResumeCheckout: () -> Unit = {},
    onBack: () -> Unit,
    onCompleteSetup: () -> Unit = {},
    onSignIn: () -> Unit,
    onActivatePromo: (String, String) -> Unit,
    onConfirmPromo: (String) -> Unit,
    onDismissPromoPreview: () -> Unit,
    onActivateStarter: () -> Unit,
    onChoose: (BillingPlan) -> Unit,
    onDemoPurchase: (String) -> Unit,
    onCancelRenewal: () -> Unit,
    onCancelPlanChange: () -> Unit = {},
    onDismissCheckoutForm: () -> Unit = {},
    onRemovePaymentMethod: () -> Unit = {},
    onContactSupport: (List<String>) -> Unit,
) {
    if (account.secureStorageProblem != null) {
        SubscriptionStorageUnavailableScreen(onBack, onSignIn)
        return
    }
    account.connectionMessage?.takeUnless { demoMode }?.let {
        SubscriptionConnectionScreen(account, state, onBack, onRetryBilling)
        return
    }
    val entryStep = subscriptionEntryStep(coreSetupReady, account.signedIn, demoMode)
    if (entryStep == SubscriptionEntryStep.SETUP) {
        SubscriptionSetupRequiredScreen(onBack, onCompleteSetup)
        return
    }
    if (entryStep == SubscriptionEntryStep.SIGN_IN) {
        SubscriptionSignInRequiredScreen(onBack, onSignIn)
        return
    }
    var confirmCancellation by remember { mutableStateOf(false) }
    var confirmCardRemoval by remember { mutableStateOf(false) }
    var confirmPlanCancellation by remember { mutableStateOf(false) }
    var demoPlan by remember { mutableStateOf<BillingPlan?>(null) }
    var promoEntry by remember { mutableStateOf(PromoCodeEntryState()) }
    var managementDialogVisible by remember { mutableStateOf(false) }
    var supportDialogVisible by remember { mutableStateOf(false) }
    var selectedOfferId by remember(
        state.catalog.plans.map(BillingPlan::offerId),
        initialSelectedPlanCode,
    ) {
        mutableStateOf(
            state.catalog.plans.firstOrNull {
                it.offerId == initialSelectedPlanCode || it.code == initialSelectedPlanCode
            }?.offerId
                ?: state.catalog.plans.firstOrNull { it.offerId == state.subscription.offerId }?.offerId
                ?: state.catalog.plans.firstOrNull(BillingPlan::recommended)?.offerId
                ?: state.catalog.plans.firstOrNull()?.offerId.orEmpty(),
        )
    }
    val program = com.govorun.lite.account.displayPromoProgram(account, state.catalog)
    val activeLimit = activeStyleLimitPresentation(account)
    val bonusLimit = bonusStyleLimitPresentation(account)
    val starterLimit = starterStyleLimitPresentation(account)
    val accessStage = subscriptionAccessStage(
        account,
        state.catalog.starterGrant,
        state.starterGrant,
    )
    val tariffFocus = remember { BringIntoViewRequester() }
    val actionScope = rememberCoroutineScope()
    val tariffState = tariffDisclosure(accessStage, initialTariffsExpanded)
    val trialDays = state.catalog.clientTrialDays.takeIf { it > 0 }
    val intro = subscriptionIntro(accessStage, com.govorun.lite.account.starterStyleLimitExhausted(account), trialDays)
    val paymentDetails = isManageableSubscription(state.subscription)
    val paymentAlert = subscriptionPaymentAlert(state.subscription, account.styleAccess).takeIf { paymentDetails }
        .takeIf { state.subscriptionDetailsAvailable }
    promoPreview?.let { preview ->
        PromoOfferConfirmationDialog(
            preview = preview,
            working = promoWorking,
            onConfirm = { onConfirmPromo(preview.id) },
            onDismiss = onDismissPromoPreview,
        )
    }
    if (promoEntry.visible) {
        PromoCodeDialog(
            code = promoEntry.code,
            working = promoWorking,
            onCodeChange = { promoEntry = promoEntry.change(it) },
            onSubmit = {
                val code = promoEntry.code
                promoEntry = promoEntry.close()
                onActivatePromo(code, selectedOfferId)
            },
            onDismiss = { promoEntry = promoEntry.close() },
        )
    }
    demoPlan?.let { plan ->
        DemoPurchaseDialog(
            plan = plan,
            onConfirm = {
                demoPlan = null
                onDemoPurchase(plan.code)
            },
            onDismiss = { demoPlan = null },
        )
    }
    if (confirmPlanCancellation) {
        AlertDialog(
            onDismissRequest = { confirmPlanCancellation = false },
            title = { Text("Отменить смену тарифа?") },
            text = { Text("Смена тарифа будет отменена. Текущий тариф сохранится.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmPlanCancellation = false
                    onCancelPlanChange()
                }, enabled = !state.working && managementActions.cancelPlanChange) { Text("Отменить смену") }
            },
            dismissButton = {
                TextButton(onClick = { confirmPlanCancellation = false }) { Text("Не менять") }
            },
        )
    }
    if (confirmCancellation) {
        AlertDialog(
            onDismissRequest = { confirmCancellation = false },
            title = { Text("Отключить подписку?") },
            text = { Text(subscriptionCancellationNotice(state.subscription)) },
            confirmButton = {
                TextButton(onClick = {
                    confirmCancellation = false
                    managementDialogVisible = true
                    onCancelRenewal()
                }, enabled = !state.working && managementActions.cancelRenewal) { Text("Отключить") }
            },
            dismissButton = {
                TextButton(onClick = { confirmCancellation = false }) { Text("Отмена") }
            },
        )
    }
    if (confirmCardRemoval) {
        RemovePaymentMethodConfirmation(
            enabled = !state.working && managementActions.removeCard,
            onConfirm = {
                confirmCardRemoval = false
                managementDialogVisible = true
                onRemovePaymentMethod()
            },
            onDismiss = {
                confirmCardRemoval = false
                managementDialogVisible = true
            },
        )
    }
    if (supportDialogVisible) {
        val details = subscriptionSupportDetails(state)
        SubscriptionHelpDialog(
            details = details,
            onContactSupport = { onContactSupport(details) },
            onDismiss = { supportDialogVisible = false },
        )
    }
    if (managementDialogVisible) {
        SubscriptionManagementDialog(
            subscription = state.subscription,
            actions = managementActions,
            working = state.working || state.loading,
            actionMessage = state.checkoutMessage.takeIf { state.managementStatusPending },
            statusAvailable = state.subscriptionDetailsAvailable,
            updating = state.subscriptionInformationUpdating,
            onRemovePaymentMethod = {
                managementDialogVisible = false
                confirmCardRemoval = true
            },
            onRefresh = onRetryBilling,
            onCancelPlanChange = { confirmPlanCancellation = true },
            onCancel = {
                managementDialogVisible = false
                confirmCancellation = true
            },
            onContactSupport = {
                managementDialogVisible = false
                supportDialogVisible = true
            },
            onDismiss = { managementDialogVisible = false },
        )
    }
    val paidAccess = accessStage == SubscriptionAccessStage.PAID_ACTIVE
    val consentVisible = hasInlineStylizationConsent()
    ProductScaffold(title = "Подписка и лимиты", onBack = onBack) { innerPadding ->
        if (consentVisible) {
            androidx.compose.foundation.lazy.LazyColumn(
                modifier = Modifier.fillMaxSize().padding(innerPadding), contentPadding = productScreenPadding(),
            ) { item { InlineStylizationConsentCard() } }
        } else LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
        ) {
            if (!program.visible && !paidAccess && paymentAlert == null && !state.messageIsPrimary) {
                item {
                    Column(
                        modifier = Modifier.productFeedCard().padding(
                            horizontal = ProductTokens.Spacing.Xl,
                            vertical = ProductTokens.Spacing.Md,
                        ),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
                    ) {
                        Text(
                            intro.title,
                            style = MaterialTheme.typography.headlineSmall,
                            textAlign = TextAlign.Center,
                        )
                        Text(
                            intro.body,
                            modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Screen),
                            style = MaterialTheme.typography.bodyMedium,
                            textAlign = TextAlign.Center,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            paymentAlert?.let { alert ->
                item {
                    SubscriptionPaymentAlertCard(
                        alert = alert,
                        onContactSupport = { supportDialogVisible = true },
                    )
                }
            }
            if (canRetryPromoAccess) item {
                TextButton(onClick = onRetryPromoAccess, enabled = !promoWorking) { Text("Проверить доступ") }
            }
            (state.checkoutMessage ?: state.message)
                ?.takeIf { state.messageIsPrimary && it.isNotBlank() }
                ?.let { message ->
                    item { SubscriptionStatusMessage(state, message, onRetryBilling, onRestartCheckout, onResumeCheckout) }
                }
            if (program.visible && !paidAccess) {
                item { PromoProgramCard(program, activeLimit, starterWorking, onActivateStarter) }
            }
            when (accessStage) {
                SubscriptionAccessStage.STARTER_OFFER -> Unit
                SubscriptionAccessStage.STARTER_ACTIVE -> item {
                    StarterActiveCard(starterLimit)
                }
                SubscriptionAccessStage.BONUS_ACTIVE -> if (account.styleAccess.type != "promo_program") item {
                    ActiveFreeAccessCard(
                        title = bonusAccessTitle(account),
                        balance = bonusLimit?.balance,
                        status = bonusLimit?.expires ?: bonusAccessStatus(account.styleAccess),
                    )
                }
                SubscriptionAccessStage.PAID_ACTIVE -> if (state.subscriptionDetailsAvailable && state.subscription.state == "refunded") {
                    item { ActiveFreeAccessCard("Подписка завершена", status = "Обновляем доступный остаток…") }
                } else {
                    val activePlan = currentAccessPlan(
                        state.catalog.plans, state.subscription, account.styleAccess,
                    )
                    item {
                        Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl)) {
                        CurrentSubscriptionCard(
                            plan = activePlan,
                            access = account.styleAccess,
                            limit = activeLimit,
                            statusText = subscriptionStatusLabel(state.subscription, state.subscriptionDetailsAvailable),
                        )
                    }
                    }
                    val proPlan = state.catalog.plans.firstOrNull {
                        it.code == "pro" && (activePlan == null || it.periodDays == activePlan.periodDays)
                    }
                    val upgradeEnabled = proPlan != null &&
                        (account.signedIn || demoMode) && billingUiEnabled &&
                        (demoMode || state.catalog.tBankPreviewEnabled) &&
                        state.purchaseAvailable
                    if (upgradeEnabled && shouldOfferProUpgrade(
                            accessType = account.styleAccess.type,
                            activePlanCode = account.styleAccess.planCode,
                            candidatePlanCode = proPlan.code,
                            periodRemainingPercent = activeLimit?.periodPercent,
                        )
                    ) {
                        item {
                            ProUpgradeAction(
                                onChoose = {
                                    onDismissCheckoutForm()
                                    selectedOfferId = proPlan.offerId
                                    actionScope.launch { tariffFocus.bringIntoView() }
                                },
                            )
                        }
                    }
                }
                SubscriptionAccessStage.PLANS -> Unit
            }
            item(key = "tariffs") {
                Column(Modifier.bringIntoViewRequester(tariffFocus)) {
                if (state.catalog.storefront.unavailable || state.catalog.plans.isEmpty()) {
                    CompactMessageCard("Тарифы временно недоступны")
                } else {
                    SubscriptionPlanSelector(
                        plans = state.catalog.plans,
                        selectedOfferId = selectedOfferId,
                        trialDays = trialDays,
                        onSelect = { onDismissCheckoutForm(); selectedOfferId = it.offerId },
                    )
                }
                }
            }
            if (!state.catalog.storefront.unavailable && tariffState.checkoutVisible) {
                state.catalog.plans.firstOrNull { it.offerId == selectedOfferId }?.let { plan ->
                    item {
                        CheckoutActionSection(
                            plan = plan,
                            trialDays = trialDays,
                            enabled = (account.signedIn || demoMode) && billingUiEnabled &&
                                (demoMode || state.catalog.tBankPreviewEnabled) && state.purchaseAvailable,
                            unavailableMessage = CHECKOUT_UNAVAILABLE_MESSAGE
                                .takeIf { checkoutUnavailable },
                            onChoose = {
                                when {
                                    onPreviewCheckout != null -> onPreviewCheckout(plan)
                                    demoMode -> demoPlan = plan
                                    else -> onChoose(plan)
                                }
                            },
                        )
                    }
                }
            }
            if (!paidAccess) state.checkoutFormFailure?.takeIf { it.offerId == selectedOfferId }?.let {
                item { CompactMessageCard(it.message) }
            }
            if (!program.visible && accessStage == SubscriptionAccessStage.STARTER_OFFER) {
                item { StarterOfferCard(working = starterWorking, onActivate = onActivateStarter) }
            }
            subscriptionRenewalNotice(state.subscription, account.styleAccess)
                ?.takeIf { state.subscriptionDetailsAvailable && paymentAlert == null && state.subscription.renewalIssue != null }
                ?.let { notice ->
                item { CompactMessageCard(notice) }
            }
            item(key = "promo-code") {
                PromoCodeAction(
                    signedIn = account.signedIn,
                    working = promoWorking,
                    onEnterCode = { promoEntry = promoEntry.open() },
                    onSignIn = onSignIn,
                )
            }
            if (paidAccess && paymentDetails) item(key = "payment-details") {
                CurrentSubscriptionPaymentCard(
                    renewalSummary = subscriptionPaymentSummary(state.subscription,
                        state.subscriptionDetailsAvailable, state.subscriptionInformationUpdating),
                    onRefresh = onRetryBilling.takeIf { !state.subscriptionDetailsAvailable && !state.loading && !state.working },
                    enabled = !state.working,
                    onManage = { managementDialogVisible = true },
                    paymentMethod = state.subscription.paymentMethod,
                    paymentStatusAvailable = state.subscriptionDetailsAvailable,
                    paymentUpdating = state.subscriptionInformationUpdating,
                )
            }
            if (!paidAccess && paymentDetails) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
                        state.subscription.paymentMethod?.presentation(state.subscriptionDetailsAvailable)?.let {
                            Text(it.text, style = MaterialTheme.typography.bodyMedium)
                        }
                        subscriptionRenewalSummary(state.subscription)?.takeIf { state.subscriptionDetailsAvailable }?.let { summary ->
                            Text(
                                summary,
                                modifier = Modifier.productFeedCard().padding(horizontal = ProductTokens.Spacing.Xl),
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        SubscriptionManagementAction(
                            enabled = !state.working,
                            onClick = { managementDialogVisible = true },
                        )
                    }
                }
            }
            promoMessage?.takeIf(String::isNotBlank)?.let { message ->
                item { SubscriptionStatusMessage(state, message, onRetryBilling, onRestartCheckout, onResumeCheckout) }
            }
            (state.checkoutMessage ?: state.message)
                ?.takeIf { !state.messageIsPrimary && it.isNotBlank() }
                ?.takeUnless { it == CHECKOUT_UNAVAILABLE_MESSAGE }
                ?.let { message ->
                item { SubscriptionStatusMessage(state, message, onRetryBilling, onRestartCheckout, onResumeCheckout) }
            }
        }
    }
}
