package com.govorun.lite.ui

import com.govorun.lite.account.BillingSubscription
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

internal fun subscriptionRenewalSummary(
    subscription: BillingSubscription,
    now: Instant = Instant.now(),
    zoneId: ZoneId = ZoneId.systemDefault(),
    locale: Locale = Locale.getDefault(),
): String? {
    if (!isManageableSubscription(subscription)) return null
    if (!subscription.renewsAutomatically) {
        return "Автопродление отключено. Следующих списаний не будет."
    }
    val next = subscription.nextCharge
    val date = next?.chargeAt?.let { runCatching { Instant.parse(it) }.getOrNull() }
    if (next == null || next.currency != "RUB" || next.amountKopeks <= 0 ||
        date == null || !date.isAfter(now)
    ) return "Автопродление включено. Дата следующего списания уточняется."
    val formatted = DateTimeFormatter.ofPattern("d MMMM yyyy, HH:mm", locale)
        .format(date.atZone(zoneId))
    return "Автопродление включено. Следующее списание — " +
        "${formatBillingKopeks(next.amountKopeks)}, $formatted."
}
