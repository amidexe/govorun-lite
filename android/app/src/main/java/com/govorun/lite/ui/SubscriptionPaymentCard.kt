package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.product.ui.ProductTokens

/** A payment method is information, not a second button inside the card. */
@Composable
internal fun SubscriptionPaymentCard(text: String, kind: String? = null) {
    Row(
        modifier = Modifier.fillMaxWidth().heightIn(min = ProductTokens.Size.Row),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxl),
    ) {
        kind?.let { PaymentSourceIcon(it) }
        Text(text, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface)
    }
}
