package com.govorun.lite.ui

import com.govorun.lite.account.BillingSubscription
import java.time.ZoneId
import java.util.Locale

internal fun subscriptionCancellationNotice(
    subscription: BillingSubscription,
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): String {
    val stopRenewal = "Списания будут отключены, карта — отвязана. "
    if (subscription.state != "trialing") {
        return stopRenewal + "Оплаченный доступ сохранится до конца периода."
    }
    val until = formatSubscriptionDate(subscription.currentPeriodEndsAt, zoneId, locale)
        ?.let { "до $it" } ?: "до конца текущего периода"
    return stopRenewal + "Пробный доступ сохранится $until. " +
        "Повторно получить бесплатный пробный период нельзя."
}
