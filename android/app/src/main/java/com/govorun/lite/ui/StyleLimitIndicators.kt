package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.product.ui.ProductTokens

@Composable
internal fun ActiveStyleLimitIndicators(
    limit: ActiveStyleLimitPresentation,
    modifier: Modifier = Modifier,
    compact: Boolean = false,
) {
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        Text(if (compact) "Лимиты" else limit.heading, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
        limit.periodRange?.takeUnless { compact }?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (limit.refreshing) {
            Text("Обновляем остаток", style = MaterialTheme.typography.bodyMedium)
        } else {
            RemainingIndicator(limit.todayLabel, limit.todayPercent, limit.todayReset)
            RemainingIndicator(limit.periodLabel, limit.periodPercent, limit.periodStatus)
        }
    }
}

@Composable
private fun RemainingIndicator(label: String, percent: Int, supporting: String?) {
    Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                label,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.bodySmall,
            )
            Text(
                "Осталось $percent%",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        LinearProgressIndicator(
            progress = { percent.coerceIn(0, 100) / 100f },
            modifier = Modifier.fillMaxWidth(),
            color = MaterialTheme.colorScheme.primary,
            trackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
        )
        supporting?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
