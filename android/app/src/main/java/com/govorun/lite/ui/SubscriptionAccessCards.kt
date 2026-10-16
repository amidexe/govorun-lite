package com.govorun.lite.ui

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.govorun.lite.account.StyleAccess
import com.govorun.lite.account.BillingPaymentMethod
import com.govorun.lite.account.presentation
import com.govorun.lite.account.BillingSubscription
import com.govorun.lite.account.BillingPlan
import com.govorun.lite.account.StarterStyleLimitPresentation
import com.product.ui.ProductTokens

@Composable
internal fun ActiveFreeAccessCard(
    title: String,
    balance: String? = null,
    status: String? = null,
) {
    BillingCard {
        Text(
            title,
            style = MaterialTheme.typography.titleMedium,
        )
        balance?.takeIf(String::isNotBlank)?.let { value ->
            Text(value, style = MaterialTheme.typography.bodyMedium)
        }
        status?.takeIf(String::isNotBlank)?.let { value ->
            Text(
                value,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
internal fun StarterActiveCard(limit: StarterStyleLimitPresentation?) {
    BillingCard {
        Text("Бесплатный лимит", style = MaterialTheme.typography.titleMedium)
        Text(
            limit?.status ?: "Обновляем остаток",
            style = MaterialTheme.typography.bodyMedium,
        )
        Text(
            "Без срока окончания. Сохраняется при подключении подписки.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun TariffCatalogStateCard(loading: Boolean, onRetry: () -> Unit) {
    BillingCard {
        Text(
            if (loading) "Загружаем тарифы…" else "Не удалось загрузить тарифы",
            style = MaterialTheme.typography.titleMedium,
        )
        if (!loading) {
            Text(
                "Проверьте подключение и повторите попытку.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            OutlinedButton(onClick = onRetry, modifier = Modifier.fillMaxWidth()) {
                Text("Повторить")
            }
        }
    }
}

internal enum class TariffCatalogContent {
    PLANS,
    LOADING,
    RETRY,
}

internal fun tariffCatalogContent(planCount: Int, loading: Boolean): TariffCatalogContent = when {
    planCount > 0 -> TariffCatalogContent.PLANS
    loading -> TariffCatalogContent.LOADING
    else -> TariffCatalogContent.RETRY
}

@Composable
internal fun StarterOfferCard(working: Boolean, onActivate: () -> Unit) {
    TextButton(onClick = onActivate, enabled = !working, modifier = Modifier.fillMaxWidth()) {
        Text(if (working) "Подключаем…" else "Попробовать без карты")
    }
}

@Composable
internal fun ProUpgradeAction(onChoose: () -> Unit) {
    OutlinedButton(
        onClick = onChoose,
        modifier = Modifier
            .fillMaxWidth()
            .padding(
                horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Sm,
            ),
    ) {
        Text("Перейти на Про")
    }
}

@Composable
internal fun CurrentSubscriptionCard(
    plan: BillingPlan?,
    access: StyleAccess,
    limit: ActiveStyleLimitPresentation?,
    statusText: String? = null,
) {
    BillingCard {
        Text("Подписка «${currentAccessTitle(plan, access)}»", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurface)
        statusText?.let { Text(it, style = MaterialTheme.typography.titleMedium) }
        if (plan != null && access.type != "plan_grant") {
            Text("${formatBillingKopeks(plan.priceKopeks)} за ${plan.billingPeriodLabel}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        subscriptionAccessEnd(access.endsAt)?.let {
            Text(it, style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }

    }
    limit?.let {
        BillingCard { ActiveStyleLimitIndicators(it, compact = true) }
    }
}

@Composable
internal fun CurrentSubscriptionPaymentCard(
    renewalSummary: String?,
    onRefresh: (() -> Unit)?,
    enabled: Boolean,
    onManage: () -> Unit,
    paymentMethod: BillingPaymentMethod?,
    paymentStatusAvailable: Boolean,
    paymentUpdating: Boolean,
) {
    BillingCard {
        Text("Способ оплаты и списания", style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
        if (paymentMethod != null || !paymentStatusAvailable) {
            val paymentText = if (!paymentStatusAvailable && !paymentUpdating) "Не удалось проверить способ оплаты"
                else paymentMethod?.presentation(paymentStatusAvailable)?.text ?: "Проверяем способ оплаты…"
            SubscriptionPaymentCard(paymentText, paymentMethod?.kind)
        }
        onRefresh?.let {
            OutlinedButton(onClick = it, modifier = Modifier.fillMaxWidth()) { Text("Проверить статус") }
        }
        renewalSummary?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
        SubscriptionActionRow("Управлять подпиской", enabled, onManage)
    }
}

internal fun currentAccessTitle(plan: BillingPlan?, access: StyleAccess): String =
    plan?.name ?: when (access.planCode) {
        "base" -> "Базовая"
        "pro" -> "Про"
        else -> "Улучшение текста"
    }

internal fun currentAccessPlan(
    plans: List<BillingPlan>, subscription: BillingSubscription, access: StyleAccess,
): BillingPlan? = plans.firstOrNull {
    access.type != "plan_grant" && it.offerId == subscription.offerId
} ?: plans.firstOrNull { it.code == access.planCode }
