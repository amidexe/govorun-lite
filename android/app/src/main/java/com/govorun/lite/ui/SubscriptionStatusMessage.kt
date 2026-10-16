package com.govorun.lite.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.govorun.lite.account.BillingUiState
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive

@Composable
internal fun SubscriptionStatusMessage(state: BillingUiState, message: String, onRetry: () -> Unit, onRestartCheckout: () -> Unit = {}, onResumeCheckout: () -> Unit = {}) {
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(state.checkoutExpiresAt) {
        while (isActive && state.checkoutExpiresAt != null) { now = System.currentTimeMillis(); delay(1_000) }
    }
    val remaining = state.checkoutExpiresAt?.let { ((it - now + 999) / 1_000).coerceAtLeast(0) }
    BillingCard {
      Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(message, style = MaterialTheme.typography.bodyMedium)
        if (remaining != null) {
            Text(if (remaining == 0L) "Срок попытки истёк. Ожидаем окончательный результат банка. Если платёж не прошёл, можно будет оформить заказ снова."
                else "Оформление доступно ещё ${remaining / 60}:${(remaining % 60).toString().padStart(2, '0')}",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (state.checkoutPending && state.checkoutResumeUrl != null && remaining != 0L) {
            OutlinedButton(onClick = onResumeCheckout, modifier = Modifier.fillMaxWidth(), enabled = !state.working) { Text("Продолжить оплату") }
        }
        if (state.checkoutRestartId != null) {
            TextButton(onClick = onRestartCheckout, modifier = Modifier.fillMaxWidth(), enabled = !state.working && !state.loading) {
                Text("Отменить оформление")
            }
        }
        if (state.checkoutRefreshFailed || state.statusUnavailable) {
            TextButton(onClick = onRetry, modifier = Modifier.fillMaxWidth(), enabled = !state.working && !state.loading) {
                Text("Проверить статус")
            }
        }
      }
    }
}
