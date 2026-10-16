package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CreditCard
import com.govorun.lite.account.AccountState
import com.product.ui.ProductTokens

@Composable
internal fun AccountSubscriptionCard(state: AccountState, onOpen: () -> Unit) {
    val access = state.styleAccess
    BillingCard {
        Column(
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xl),
            ) {
                Surface(shape = CircleShape, color = MaterialTheme.colorScheme.primaryContainer) {
                    Icon(
                        PhosphorIcons.Regular.CreditCard,
                        contentDescription = null,
                        modifier = Modifier.padding(ProductTokens.Spacing.Lg).size(ProductTokens.Size.SlotIcon),
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs)) {
                    Text("Подписка и лимиты", style = MaterialTheme.typography.titleMedium)
                    Text(
                        accountSubscriptionTitle(state),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (state.hasEntitlement) subscriptionAccessEnd(access.endsAt)?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
            OutlinedButton(onClick = onOpen, modifier = Modifier.fillMaxWidth()) {
                Text("Управлять подпиской")
            }
        }
    }
}

internal fun accountSubscriptionTitle(state: AccountState): String = when {
    !state.hasEntitlement -> "Тарифы, карта и списания"
    state.styleAccess.type == "promo_program" -> "Промо"
    state.styleAccess.type == "starter" -> "Бесплатный лимит"
    state.styleAccess.balanceKind == "single_bonus" -> "Бонусный лимит"
    state.styleAccess.planCode == "pro" -> "Про"
    else -> "Базовая"
}
