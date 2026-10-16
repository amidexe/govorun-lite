package com.govorun.lite.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.govorun.lite.account.AccountState
import com.govorun.lite.account.BillingUiState
import com.govorun.lite.account.connectionHint
import com.govorun.lite.account.connectionMessage
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding

@Composable
internal fun SubscriptionConnectionScreen(
    account: AccountState,
    state: BillingUiState,
    onBack: () -> Unit,
    onRetry: () -> Unit,
) {
    val busy = state.working || state.loading
    BackHandler(onBack = onBack)
    ProductScaffold(title = "Подписка и лимиты", onBack = onBack) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding)
                .verticalScroll(rememberScrollState()).padding(productScreenPadding()),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Section),
        ) {
            BillingCard {
                Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
                    Text(
                        if (busy && account.networkAvailable) "Проверяем доступ…"
                        else account.connectionMessage.orEmpty(),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(if (busy && account.networkAvailable) "Диктовка работает офлайн. Дождитесь проверки доступа."
                        else account.connectionHint, style = MaterialTheme.typography.bodyMedium)
                    if (state.checkoutPurchasePending) {
                        Text("Статус начатой оплаты пока не проверен. Повторять оплату не нужно.",
                            style = MaterialTheme.typography.bodyMedium)
                    }
                    OutlinedButton(onClick = onRetry, enabled = !busy, modifier = Modifier.fillMaxWidth()) {
                        Text(if (busy) "Проверяем…" else "Повторить")
                    }
                }
            }
        }
    }
}
