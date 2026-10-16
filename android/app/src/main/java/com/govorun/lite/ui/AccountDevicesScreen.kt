package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.DeviceMobile
import com.govorun.lite.account.AccountState
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding

@Composable
internal fun AccountDevicesSummaryCard(state: AccountState, onOpen: () -> Unit) {
    BillingCard {
        Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
            ) {
                Surface(shape = CircleShape, color = MaterialTheme.colorScheme.primaryContainer) {
                    Icon(
                        PhosphorIcons.Regular.DeviceMobile,
                        contentDescription = null,
                        modifier = Modifier.padding(ProductTokens.Spacing.Lg).size(ProductTokens.Size.SlotIcon),
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs),
                ) {
                    Text("Устройства", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Подключено ${state.devices.size} из ${state.deviceLimit}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            OutlinedButton(onClick = onOpen, modifier = Modifier.fillMaxWidth()) {
                Text("Список устройств")
            }
        }
    }
}

@Composable
internal fun AccountDevicesScreen(
    state: AccountState,
    onBack: () -> Unit,
    onRetry: () -> Unit,
    onRevokeDevice: (String) -> Unit,
) {
    ProductScaffold(title = "Список устройств", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
        ) {
            if (state.syncing || state.message != null) {
                item {
                    BillingCard {
                        Text(
                            state.message ?: "Обновляем список устройств…",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (state.message != null) {
                            OutlinedButton(
                                onClick = onRetry,
                                enabled = !state.syncing,
                                modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
                            ) { Text("Обновить список") }
                        }
                    }
                }
            }
            item { DevicesCard(state, onReplaceDevice = {}, onRevokeDevice) }
        }
    }
}
