package com.govorun.lite.ui.home

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.ChartLineUp
import com.product.ui.ProductTokens
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlin.math.max

@Composable
internal fun StatisticsCard(
    state: StatisticsState,
    syncDecisionPending: Boolean = false,
    onEvent: (HomeEvent) -> Unit = {},
) {
    var expanded by rememberSaveable { mutableStateOf(false) }
    val summary = state.toSummary()

    Card(
        modifier = Modifier
            .feedCard()
            .animateContentSize(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column {
            FeedSectionHeader(
                title = "Статистика",
                icon = PhosphorIcons.Regular.ChartLineUp,
                trailingText = if (expanded) null else "${summary.weekWords.wordsLabel()} за 7 дней",
                expanded = expanded,
                onToggle = { expanded = !expanded },
            )
            AnimatedVisibility(expanded) {
                Column {
                    CompactDivider()
                    if (syncDecisionPending) {
                        Column(Modifier.padding(ProductTokens.Spacing.Section)) {
                            com.govorun.lite.ui.DataSyncDecision(
                                onMergeLocalData = { onEvent(HomeEvent.MergeAccountData) },
                                onKeepLocalData = { onEvent(HomeEvent.KeepAccountDataLocal) },
                            )
                        }
                        CompactDivider()
                    }
                    WeekSummary(state)
                    if (summary.hasWeeklyActivity) {
                        CompactDivider()
                        WeeklyActivityChart(state.wordHistory)
                    }
                }
            }
            if (syncDecisionPending && !expanded) {
                TextButton(
                    onClick = { expanded = true },
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Lg),
                ) { Text("Сохранить данные в аккаунте") }
            }
        }
    }
}

@Composable
private fun WeekSummary(state: StatisticsState) {
    val summary = state.toSummary()
    Column(
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        modifier = Modifier.padding(ProductTokens.Spacing.Section),
    ) {
        Row(
            verticalAlignment = Alignment.Bottom,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs),
                modifier = Modifier.weight(1f),
            ) {
                Text(
                    "За 7 дней",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    summary.weekWords.wordsLabel(),
                    style = MaterialTheme.typography.titleMedium,
                )
            }
            Column(
                horizontalAlignment = Alignment.End,
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xs),
            ) {
                Text(
                    "Сегодня",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    summary.todayWords.wordsLabel(),
                    style = MaterialTheme.typography.titleLarge,
                )
            }
        }
        Text(
            summary.comparison,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Время речи за 7 дней: ${summary.weekSpeechSeconds.speechDurationLabel()}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "За всё время: ${summary.totalWords.wordsLabel()} · " +
                "${summary.totalSpeechSeconds.speechDurationLabel()} речи",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            "Последние 7 дней, включая сегодня",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun WeeklyActivityChart(values: List<Long>) {
    val normalized = values.takeLast(7).let { List(7 - it.size) { 0L } + it }
    val locale = Locale.forLanguageTag("ru")
    val formatter = DateTimeFormatter.ofPattern("EE", locale)
    val labels = (6 downTo 0).map { daysAgo ->
        LocalDate.now()
            .minusDays(daysAgo.toLong())
            .format(formatter)
            .removeSuffix(".")
            .lowercase(locale)
    }
    val peak = max(1L, normalized.maxOrNull() ?: 1L)
    val lineColor = MaterialTheme.colorScheme.primary
    val gridColor = MaterialTheme.colorScheme.outlineVariant.copy(
        alpha = ProductTokens.Alpha.Divider,
    )
    val accessibilitySummary = labels.zip(normalized)
        .joinToString { (day, value) -> "$day — $value слов" }

    Column(
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
        modifier = Modifier.padding(ProductTokens.Spacing.Section),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "Слова по дням",
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.bodyMedium,
            )

        }
        Canvas(
            modifier = Modifier
                .fillMaxWidth()
                .height(92.dp)
                .semantics {
                    contentDescription = "Активность за неделю: $accessibilitySummary"
                },
        ) {
            val horizontalInset = 5.dp.toPx()
            val top = 8.dp.toPx()
            val bottom = size.height - 4.dp.toPx()
            val width = size.width - horizontalInset * 2
            val step = width / (normalized.size - 1)
            fun pointX(index: Int) = horizontalInset + step * index
            fun pointY(value: Long) =
                bottom - (bottom - top) * (value.toFloat() / peak.toFloat())

            drawLine(
                color = gridColor,
                start = androidx.compose.ui.geometry.Offset(horizontalInset, bottom),
                end = androidx.compose.ui.geometry.Offset(size.width - horizontalInset, bottom),
                strokeWidth = 1.dp.toPx(),
            )
            drawLine(
                color = gridColor.copy(alpha = gridColor.alpha * 0.55f),
                start = androidx.compose.ui.geometry.Offset(
                    horizontalInset,
                    (top + bottom) / 2f,
                ),
                end = androidx.compose.ui.geometry.Offset(
                    size.width - horizontalInset,
                    (top + bottom) / 2f,
                ),
                strokeWidth = 1.dp.toPx(),
            )

            val line = Path()
            normalized.forEachIndexed { index, value ->
                val x = pointX(index)
                val y = pointY(value)
                if (index == 0) line.moveTo(x, y) else line.lineTo(x, y)
            }
            val area = Path().apply {
                moveTo(pointX(0), bottom)
                normalized.forEachIndexed { index, value ->
                    lineTo(pointX(index), pointY(value))
                }
                lineTo(pointX(normalized.lastIndex), bottom)
                close()
            }
            drawPath(area, color = lineColor.copy(alpha = 0.09f))
            drawPath(
                line,
                color = lineColor,
                style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round),
            )
            normalized.forEachIndexed { index, value ->
                drawCircle(
                    color = lineColor,
                    radius = if (index == normalized.lastIndex) {
                        4.dp.toPx()
                    } else {
                        2.5.dp.toPx()
                    },
                    center = androidx.compose.ui.geometry.Offset(
                        pointX(index),
                        pointY(value),
                    ),
                )
            }
        }
        Row(
            horizontalArrangement = Arrangement.SpaceBetween,
            modifier = Modifier.fillMaxWidth(),
        ) {
            labels.forEach { label ->
                Text(
                    label,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
