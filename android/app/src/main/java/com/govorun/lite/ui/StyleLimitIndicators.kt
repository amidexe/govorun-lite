package com.govorun.lite.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CaretDown
import com.adamglin.phosphoricons.regular.CaretUp
import com.govorun.lite.account.ActiveStyleLimitPresentation
import com.product.ui.ProductTokens

@Composable
internal fun ActiveStyleLimitIndicators(
    limit: ActiveStyleLimitPresentation,
    modifier: Modifier = Modifier,
    compact: Boolean = false,
) {
    val context = LocalContext.current
    // A product's own limit appears only for people who let its app use Govorun (Эхо today).
    val products = remember(limit.products) { limit.products.filter { com.govorun.lite.api.smart.SmartProducts.connected(context, it.product) } }
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        if (!compact) Text(limit.heading, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
        if (limit.refreshing) {
            Text("Обновляем остаток", style = MaterialTheme.typography.bodyMedium)
        } else {
            // Today only, one thin bar per product: the limit is a guard, not something to watch. The 30 days speak up
            // in one line only when little is left.
            LimitRow("Улучшение текста", limit.todayLeft, limit.todayLimit, limit.todayPercent)
            products.forEach { LimitRow(it.title, it.todayLeft, it.todayLimit, it.todayPercent) }
            // The 30 days wait behind «30 дней», as Statistics' details do; they open by themselves when little is left.
            val low = (limit.periodPercent <= 20) || products.any { (it.periodPercent ?: 100) <= 20 }
            var open by androidx.compose.runtime.saveable.rememberSaveable { androidx.compose.runtime.mutableStateOf(false) }
            Row(
                Modifier.fillMaxWidth().clickable { open = !open },
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(limit.resetLine.orEmpty(), modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("30 дней", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Icon(if (open || low) PhosphorIcons.Regular.CaretUp else PhosphorIcons.Regular.CaretDown, contentDescription = null,
                    modifier = Modifier.padding(start = 4.dp).size(14.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (open || low) {
                Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs)) {
                    listOfNotNull(
                        periodLine("Улучшение текста", limit.periodLeft, limit.periodLimit),
                    ).plus(products.mapNotNull { periodLine(it.title, it.periodLeft, it.periodLimit) })
                        .plus(listOfNotNull(limit.periodResetLine)).forEach { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                }
            }
        }
    }
}

/** «Диктовка — 988 из 1 000 за 30 дней». */
private fun periodLine(who: String, left: Int?, limit: Int?): String? =
    if (left == null || limit == null) null else "$who — ${count(left.coerceIn(0, limit))} из ${count(limit)} за 30 дней"

/** «Диктовка ··· 38 из 50 на сегодня» over a thin bar; percent only when the counts are unknown. */
@Composable
private fun LimitRow(label: String, left: Int?, limit: Int?, percent: Int) {
    Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Text(
                "осталось $percent% на сегодня",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        LinearProgressIndicator(
            progress = { percent.coerceIn(0, 100) / 100f },
            modifier = Modifier.fillMaxWidth().height(4.dp),
            color = MaterialTheme.colorScheme.primary,
            trackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
            drawStopIndicator = {},
        )
    }
}

/** «1 000» with a thin space, as Russian writes thousands. */
private fun count(value: Int): String = "%,d".format(java.util.Locale.forLanguageTag("ru"), value).replace('\u00A0', '\u2009')
