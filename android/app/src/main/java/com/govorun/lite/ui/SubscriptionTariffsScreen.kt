package com.govorun.lite.ui

import com.govorun.lite.model.billing.unavailable
import com.govorun.lite.model.billing.unavailableMessage
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.adamglin.phosphoricons.Regular
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.regular.CaretRight
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import com.govorun.lite.account.BillingPlan
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.connectionMessage
import com.govorun.lite.account.purchaseAvailable
import com.govorun.lite.account.BillingUiState
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding

@Composable
internal fun SubscriptionTariffsScreen(
    state: BillingUiState,
    account: AccountState,
    selectedOfferId: String,
    enabled: Boolean,
    onSelect: (BillingPlan) -> Unit,
    onBack: () -> Unit,
    onRetry: () -> Unit,
    onRestartCheckout: () -> Unit = {},
    onResumeCheckout: () -> Unit = {},
    onChoose: (BillingPlan) -> Unit,
    promoAction: @Composable () -> Unit,
) {
    if (account.connectionMessage != null) {
        SubscriptionConnectionScreen(account, state, onBack, onRetry)
        return
    }
    val trialDays = state.catalog.clientTrialDays.takeIf { it > 0 }
    BackHandler(onBack = onBack)
    ProductScaffold(title = "Выбор тарифа", onBack = onBack) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding)
                .verticalScroll(rememberScrollState()).padding(productScreenPadding()),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Section),
        ) {
            if (state.catalog.storefront.unavailable) {
                CompactMessageCard(state.catalog.storefront.unavailableMessage)
            } else when (tariffCatalogContent(state.catalog.plans.size, state.loading)) {
                TariffCatalogContent.PLANS -> SubscriptionPlanSelector(
                    plans = state.catalog.plans,
                    selectedOfferId = selectedOfferId,
                    trialDays = trialDays,
                    onSelect = onSelect,
                )
                TariffCatalogContent.LOADING -> TariffCatalogStateCard(true, onRetry)
                TariffCatalogContent.RETRY -> TariffCatalogStateCard(false, onRetry)
            }
            val plan = state.catalog.plans.firstOrNull { it.offerId == selectedOfferId }
            val current = subscriptionOfferIsCurrent(state, selectedOfferId)
            plan?.let {
                CheckoutActionSection(
                    plan = it,
                    trialDays = trialDays,
                    enabled = enabled && state.purchaseAvailable && !current,
                    onChoose = { onChoose(it) },
                    actionTitle = when {
                        state.working -> "Проверяем условия…"
                        current -> "Текущий тариф"
                        trialDays != null -> "Попробовать бесплатно"
                        state.subscription.renewsAutomatically -> "Сменить тариф"
                        else -> "Подключить подписку"
                    },
                )
            }
            state.checkoutFormFailure?.takeIf { it.offerId == selectedOfferId }?.let {
                Text(it.message, style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error)
            }
            promoAction()
            (state.checkoutMessage ?: state.message)?.takeIf(String::isNotBlank)?.let {
                SubscriptionStatusMessage(state, it, onRetry, onRestartCheckout, onResumeCheckout)
            }
        }
    }
}

internal fun subscriptionOfferIsCurrent(state: BillingUiState, offerId: String): Boolean =
    state.subscription.state in setOf("active", "trialing") &&
        state.subscription.offerId == offerId && state.subscription.renewsAutomatically && state.subscription.paymentMethod?.available != false

@Composable
internal fun SubscriptionActionRow(title: String, enabled: Boolean, onClick: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth()
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .heightIn(min = 52.dp)
            .padding(vertical = ProductTokens.Spacing.Sm),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        Text(title, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = if (enabled) 1f else 0.38f))
        Icon(PhosphorIcons.Regular.CaretRight, contentDescription = null,
            modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = if (enabled) 1f else 0.38f))
    }
}
