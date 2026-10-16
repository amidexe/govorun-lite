package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import com.govorun.lite.account.BillingPaymentMethod
import com.govorun.lite.account.presentation
import com.product.ui.ProductTokens

@Composable
internal fun SubscriptionPaymentMethodActions(
    method: BillingPaymentMethod?,
    statusAvailable: Boolean,
    working: Boolean,
    onRemove: () -> Unit,
    onRefresh: () -> Unit,
    canRemove: Boolean,
    updating: Boolean = false,
) {
    val card = method?.presentation(statusAvailable)
    Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
        if (card != null || !statusAvailable) {
            val text = if (!statusAvailable && !updating) "Не удалось проверить способ оплаты"
                else card?.text ?: "Проверяем способ оплаты…"
            SubscriptionPaymentCard(text, method?.kind)
        }
        if (canRemove) {
            OutlinedButton(onClick = onRemove, enabled = !working, modifier = Modifier.fillMaxWidth()) {
                Text(if (method?.kind == "sbp") "Отключить СБП" else "Отвязать карту", textAlign = TextAlign.Center, style = MaterialTheme.typography.labelLarge)
            }
        }
        if (!working && !updating && (card?.needsRefresh == true || !statusAvailable)) {
            OutlinedButton(onClick = onRefresh, modifier = Modifier.fillMaxWidth().heightIn(min = ProductTokens.Size.TouchTarget)) {
                Text(if (method?.state == "removal_pending") "Проверить отвязку" else "Проверить статус")
            }
        }
        if (statusAvailable && method?.state == "none") {
            Text(
                "При новом подключении подписки можно выбрать способ оплаты.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
internal fun RemovePaymentMethodConfirmation(
    enabled: Boolean,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Отключить способ оплаты и списания?") },
        text = {
            Text("Оплаченный доступ сохранится до конца периода. " +
                "При новом подключении подписки можно выбрать способ оплаты.")
        },
        confirmButton = {
            TextButton(onClick = onConfirm, enabled = enabled) { Text("Отключить") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Оставить подключённым") } },
    )
}
