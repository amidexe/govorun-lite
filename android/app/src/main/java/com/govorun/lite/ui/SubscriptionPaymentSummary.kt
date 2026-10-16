package com.govorun.lite.ui

import com.govorun.lite.account.BillingSubscription

/** Access may outlive a cancelled or missing subscription. Unknown billing is a third state. */
internal const val SUBSCRIPTION_UPDATING = "Обновляем информацию о подписке…"

internal fun subscriptionPaymentSummary(
    subscription: BillingSubscription,
    statusAvailable: Boolean,
    updating: Boolean = false,
): String =
    when {
        !statusAvailable && updating -> SUBSCRIPTION_UPDATING
        !statusAvailable -> "Не удалось проверить списания. Обновите состояние подписки."
        subscription.state == "refunded" -> "Возврат выполнен. Автопродление отключено. Списаний не будет."
        !subscription.renewsAutomatically -> "Автопродление отключено. Списаний не будет."
        else -> subscriptionRenewalSummary(subscription)
            ?: "Автопродление включено. Дата следующего списания уточняется."
    }

internal fun subscriptionStatusLabel(subscription: BillingSubscription, statusAvailable: Boolean): String? =
    if (!statusAvailable) null else when (subscription.state) {
        "active" -> "Подписка активна"
        "trialing" -> "Пробный период"
        "cancelled" -> "Автопродление отключено"
        "refunded" -> "Подписка завершена после возврата"
        "expired" -> "Подписка завершена"
        "grace" -> "Ожидаем оплату продления"
        else -> null
    }

internal fun subscriptionAccessEnd(value: String): String? = runCatching {
    val date = java.time.Instant.parse(value).atZone(java.time.ZoneId.systemDefault())
    "Доступ до " + java.time.format.DateTimeFormatter.ofPattern("d MMMM yyyy", java.util.Locale.forLanguageTag("ru-RU")).format(date)
}.getOrNull()
