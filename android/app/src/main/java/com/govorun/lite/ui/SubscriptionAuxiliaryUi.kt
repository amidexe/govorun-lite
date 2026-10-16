package com.govorun.lite.ui

import com.govorun.lite.account.active
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import com.govorun.lite.account.AccountHttpException
import com.govorun.lite.account.BillingPlan
import com.govorun.lite.account.BillingSubscription
import com.govorun.lite.account.PromoCodeInputException
import com.govorun.lite.account.StyleAccess
import com.product.ui.ProductTokens
import com.product.ui.productFeedCard
import java.io.IOException
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import org.json.JSONObject

@Composable
internal fun SubscriptionManagementAction(
    enabled: Boolean,
    onClick: () -> Unit,
) {
    OutlinedButton(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier
            .productFeedCard()
            .heightIn(min = ProductTokens.Size.TouchTarget),
    ) { Text("Управлять подпиской") }
}

@Composable
internal fun TariffDiscoveryAction(onClick: () -> Unit) {
    TextButton(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
    ) { Text("Посмотреть тарифы") }
}

@Composable
internal fun SubscriptionManagementDialog(
    subscription: BillingSubscription,
    actions: com.govorun.lite.account.BillingActions,
    working: Boolean,
    onCancel: () -> Unit,
    onContactSupport: () -> Unit,
    onDismiss: () -> Unit,
    onCancelPlanChange: () -> Unit = {},
    actionMessage: String? = null,
    statusAvailable: Boolean = true,
    updating: Boolean = false,
    onRemovePaymentMethod: () -> Unit = {},
    onRefresh: () -> Unit = {},
) {
    val summary = subscriptionManagementSummary(subscription, statusAvailable = statusAvailable)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Управление подпиской", style = MaterialTheme.typography.titleLarge) },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
            ) {
                if (statusAvailable && subscription.planCode.isNotBlank()) {
                    Text(if (subscription.planCode == "pro") "Про" else "Базовая",
                        style = MaterialTheme.typography.titleMedium)
                }
                if (statusAvailable) {
                    SubscriptionManagementCharge(subscription, statusAvailable, updating)
                }
                if (statusAvailable && subscription.state != "pending") {
                    summary.periodEnd?.let {
                        Text("${summary.periodLabel} $it", style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                if (statusAvailable) {
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                }
                SubscriptionPaymentMethodActions(
                    method = subscription.paymentMethod,
                    statusAvailable = statusAvailable,
                    working = working,
                    onRemove = onRemovePaymentMethod,
                    onRefresh = onRefresh,
                    canRemove = actions.removeCard,
                    updating = updating,
                )
                if (actions.cancelRenewal) {
                    OutlinedButton(
                        onClick = onCancel,
                        enabled = !working,
                        modifier = Modifier.fillMaxWidth().heightIn(min = ProductTokens.Size.TouchTarget),
                    ) { Text("Отключить подписку", style = MaterialTheme.typography.labelLarge) }
                }
                if (actions.cancelPlanChange) {
                    Text("Запланирована смена тарифа")
                    OutlinedButton(onClick = onCancelPlanChange, enabled = !working) {
                        Text("Отменить смену тарифа")
                    }
                }
                actionMessage?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
            }
        },
        dismissButton = {
            TextButton(onClick = onContactSupport, enabled = !working) { Text("Поддержка") }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Закрыть") } },
    )
}

@Composable
internal fun SubscriptionHelpDialog(
    details: List<String>,
    onContactSupport: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Поддержка по подписке") },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
            ) {
                Text(
                    SUBSCRIPTION_SUPPORT_EMAIL,
                    style = MaterialTheme.typography.bodyMedium,
                )
                Text(
                    details.joinToString("\n"),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Text(
                    "Вопросы об оплате, отмене или возврате? Напишите в поддержку — " +
                        "мы проверим платёж и ответим.",
                    style = MaterialTheme.typography.bodyMedium,
                )
                OutlinedButton(
                    onClick = onContactSupport,
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = ProductTokens.Size.TouchTarget),
                ) { Text("Написать в поддержку") }
                TextButton(
                    onClick = onDismiss,
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = ProductTokens.Size.TouchTarget),
                ) { Text("Закрыть") }
            }
        },
        confirmButton = {},
    )
}

@Composable
private fun SubscriptionInfoRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.width(ProductTokens.Spacing.Section))
        Text(
            value,
            modifier = Modifier.weight(1f),
            textAlign = TextAlign.End,
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}

internal data class SubscriptionManagementSummary(
    val status: String,
    val periodLabel: String,
    val periodEnd: String?,
    val renewal: String,
)

internal data class SubscriptionPaymentAlert(
    val title: String,
    val body: String,
)

internal fun isManageableSubscription(subscription: BillingSubscription): Boolean =
    com.govorun.lite.shared.SharedBusiness.evaluate(org.json.JSONObject().put("operation", "billingManagementVisible")
        .put("subscription", org.json.JSONObject().put("state", subscription.state)
            .put("renewsAutomatically", subscription.renewsAutomatically).put("paymentMethod",
                subscription.paymentMethod?.let { org.json.JSONObject().put("state", it.state) } ?: org.json.JSONObject.NULL)))
        .getBoolean("visible")

internal fun subscriptionManagementSummary(
    subscription: BillingSubscription,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
    statusAvailable: Boolean = true,
): SubscriptionManagementSummary = if (!statusAvailable) {
    SubscriptionManagementSummary("Уточняется", "", null, "Уточняется")
} else SubscriptionManagementSummary(
    status = when (subscription.state) {
        "pending" -> "Запланирована"
        "trialing" -> "Пробный период"
        "active" -> "Активна"
        "grace" -> "Нужна оплата"
        else -> "Неактивна"
    },
    periodLabel = when (subscription.state) {
        "pending" -> "Первое списание"
        "trialing" -> "Пробный доступ до"
        else -> "Действует до"
    },
    periodEnd = formatSubscriptionDate(subscription.currentPeriodEndsAt, zoneId, locale),
    renewal = if (subscription.paymentMethod?.available == false) "Приостановлено" else if (subscription.renewsAutomatically) "Включено" else "Отключено",
)

internal fun formatSubscriptionDate(value: String, zoneId: ZoneId, locale: Locale): String? =
    runCatching {
        DateTimeFormatter.ofPattern("d MMMM yyyy", locale)
            .format(Instant.parse(value).atZone(zoneId))
    }.getOrNull()

internal fun subscriptionPaymentAlert(
    subscription: BillingSubscription,
    currentAccess: StyleAccess? = null,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): SubscriptionPaymentAlert? {
    val issue = subscription.renewalIssue ?: return null
    if (issue.paymentState != "not_charged" ||
        issue.amountKopeks <= 0 || issue.currency != "RUB"
    ) return null
    val body = when (issue.state) {
        "retry_scheduled" -> "Деньги не списаны. Повторим оплату автоматически."
        "grace_ending", "stopped" -> if (
            issue.resolutionKind == "customer_action_required"
        ) {
            "Деньги не списаны. Откройте подписку, чтобы продолжить."
        } else {
            "Деньги не списаны. Мы уже проверяем проблему."
        }
        else -> "Деньги не списаны. Мы уже проверяем проблему."
    }
    return SubscriptionPaymentAlert("Оплата не прошла", body)
}

internal fun subscriptionRenewalNotice(
    subscription: BillingSubscription,
    currentAccess: StyleAccess? = null,
    now: Instant = Instant.now(),
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): String? {
    if (subscription.paymentMethod?.available == false) return "Способ оплаты временно недоступен. Новых списаний не будет — выберите другой способ."
    subscriptionPaymentAlert(subscription, currentAccess, zoneId, locale)?.let { alert ->
        return "${alert.title}. ${alert.body}"
    }
    subscription.renewalIssue?.let { issue ->
        val access = renewalAccessCopy(
            issueAccessEndsAt = issue.accessEndsAt,
            currentAccess = currentAccess,
            verb = "работает",
            zoneId = zoneId,
            locale = locale,
        )
        return when (issue.state) {
            "retry_scheduled" -> buildString {
                append("Оплата не прошла. Следующая попытка — завтра.")
                append(access)
            }
            "grace_ending" -> buildString {
                append("Автопродление отключено.")
                append(access)
            }
            "stopped" -> "Подписка остановлена. Возобновить её можно вручную."
            else -> null
        }
    }
    val next = subscription.nextCharge ?: return null
    if (next.amountKopeks <= 0 || next.currency != "RUB") return null
    val noticeAt = runCatching { Instant.parse(next.noticeAt) }.getOrNull() ?: return null
    val chargeAt = runCatching { Instant.parse(next.chargeAt) }.getOrNull() ?: return null
    if (now.isBefore(noticeAt) || !now.isBefore(chargeAt)) return null
    return "Завтра продлится подписка — ${formatBillingKopeks(next.amountKopeks)}."
}

private fun renewalAccessCopy(
    issueAccessEndsAt: String,
    currentAccess: StyleAccess?,
    verb: String,
    zoneId: ZoneId,
    locale: Locale,
): String {
    val issueEnd = runCatching { Instant.parse(issueAccessEndsAt) }.getOrNull()
    val currentEnd = currentAccess
        ?.takeIf { it.active }
        ?.endsAt
        ?.let { value -> runCatching { Instant.parse(value) }.getOrNull() }
    val useCurrentAccess = issueEnd != null && currentEnd != null && currentEnd.isAfter(issueEnd)
    val selected = if (useCurrentAccess) currentEnd else issueEnd
    val date = selected?.let {
        DateTimeFormatter.ofPattern("d MMMM yyyy", locale).format(it.atZone(zoneId))
    } ?: return ""
    val currentAccessIsAdditional = currentAccess?.type !in setOf("paid", "trial", "plan_grant")
    return if (useCurrentAccess && currentAccessIsAdditional) {
        " Бонусный лимит действует до $date."
    } else {
        " Улучшение текста $verb до $date."
    }
}

@Composable
internal fun SubscriptionPaymentAlertCard(
    alert: SubscriptionPaymentAlert,
    onContactSupport: () -> Unit,
) {
    Card(
        modifier = Modifier.productFeedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column(
            modifier = Modifier.padding(ProductTokens.Spacing.Section),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            Text(alert.title, style = MaterialTheme.typography.titleMedium)
            Text(alert.body, style = MaterialTheme.typography.bodyMedium)
            TextButton(onClick = onContactSupport) { Text("Написать в поддержку") }
        }
    }
}

@Composable
internal fun CompactMessageCard(message: String) {
    BillingCard {
        Text(message, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
internal fun DemoPurchaseDialog(
    plan: BillingPlan,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(planName(plan)) },
        text = { Text("Тестовая покупка без списания денег.") },
        confirmButton = { TextButton(onClick = onConfirm) { Text("Продолжить") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Отмена") } },
    )
}

@Composable
internal fun BillingCard(
    modifier: Modifier = Modifier.productFeedCard(),
    border: BorderStroke? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        border = border,
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column(modifier = Modifier.padding(ProductTokens.Spacing.Section), content = content)
    }
}

internal fun planName(plan: BillingPlan): String = plan.name

internal fun promoCampaignMessage(error: Throwable): String =
    com.govorun.lite.account.SharedPromotionRules.message(error)

internal const val SUBSCRIPTION_SUPPORT_EMAIL = "support@govorun.club"
