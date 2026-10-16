package com.govorun.lite.ui.home

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Sparkle
import com.product.ui.ProductTokens

@Composable
internal fun StarterGuidanceCard(onEvent: (HomeEvent) -> Unit) {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer,
        ),
    ) {
        Column(modifier = Modifier.padding(ProductTokens.Spacing.Section)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    PhosphorIcons.Regular.Sparkle,
                    contentDescription = null,
                    modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                    tint = MaterialTheme.colorScheme.primary,
                )
                Spacer(Modifier.width(ProductTokens.Spacing.Lg))
                Text("Выберите стиль", style = MaterialTheme.typography.titleMedium)
            }
            Text(
                "Выберите стиль и получите результат, который вам больше нравится.",
                modifier = Modifier.padding(top = ProductTokens.Spacing.Lg),
                style = MaterialTheme.typography.bodyMedium,
            )
            TextButton(
                onClick = { onEvent(HomeEvent.DismissStarterGuidance) },
                modifier = Modifier.align(Alignment.End),
            ) { Text("Понятно") }
        }
    }
}
