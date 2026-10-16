package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import com.govorun.lite.account.BillingSubscription
import com.product.ui.ProductTokens
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

@Composable
internal fun SubscriptionManagementCharge(subscription: BillingSubscription, statusAvailable: Boolean, updating: Boolean) {
    val next = subscription.nextCharge
    val date = next?.chargeAt?.let { runCatching { Instant.parse(it) }.getOrNull() }
    if (statusAvailable && subscription.renewsAutomatically && subscription.renewalIssue == null &&
        next != null && next.currency == "RUB" && next.amountKopeks > 0 && date != null && date.isAfter(Instant.now())) {
        Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
            Text(if (subscription.state == "pending") "Первое списание" else "Следующее списание",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(formatBillingKopeks(next.amountKopeks), style = MaterialTheme.typography.titleMedium)
            Text(DateTimeFormatter.ofPattern("d MMMM yyyy, HH:mm", Locale.forLanguageTag("ru-RU"))
                .format(date.atZone(ZoneId.systemDefault())), style = MaterialTheme.typography.bodyMedium)
        }
    } else {
        Text(subscriptionPaymentSummary(subscription, statusAvailable, updating),
            style = MaterialTheme.typography.bodyMedium)
    }
}
