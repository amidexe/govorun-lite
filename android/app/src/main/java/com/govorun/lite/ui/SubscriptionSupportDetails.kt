package com.govorun.lite.ui

import com.govorun.lite.account.BillingUiState
import java.time.Instant
import java.time.ZoneId
import java.util.Locale

/** Only visible subscription information belongs in a user-reviewable support draft. */
internal fun subscriptionSupportDetails(
    state: BillingUiState,
    now: Instant = Instant.now(),
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): List<String> {
    val subscription = state.subscription
    if (subscription.state == "none" && !subscription.renewsAutomatically) {
        return listOf(if (state.loading) "Сведения о подписке обновляются." else "Подписка: не подключена")
    }
    val plan = state.catalog.plans.firstOrNull { it.code == subscription.planCode }
        ?.name?.takeIf(String::isNotBlank) ?: when (subscription.planCode) {
        "base" -> "Базовая"
        "pro" -> "Про"
        else -> "Не определён"
    }
    val summary = subscriptionManagementSummary(subscription, zoneId, locale)
    return buildList {
        add("Тариф: $plan")
        add("Статус: ${summary.status}")
        summary.periodEnd?.let { add("${summary.periodLabel}: $it") }
        subscriptionRenewalSummary(subscription, now, zoneId, locale)?.let(::add)
        subscriptionPaymentAlert(subscription)?.let { add("${it.title}. ${it.body}") }
    }
}
