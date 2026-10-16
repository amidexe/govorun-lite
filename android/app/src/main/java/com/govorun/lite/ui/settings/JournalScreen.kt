package com.govorun.lite.ui.settings

import androidx.compose.ui.unit.dp

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Info
import com.adamglin.phosphoricons.regular.CheckCircle
import com.adamglin.phosphoricons.regular.CaretDown
import com.adamglin.phosphoricons.regular.CaretRight
import com.adamglin.phosphoricons.regular.Copy
import com.adamglin.phosphoricons.regular.Trash
import com.adamglin.phosphoricons.regular.WarningCircle
import com.product.ui.FeedSectionHeader
import com.product.ui.FeedCard
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTopAction
import com.product.ui.ProductTokens
import com.product.ui.productScreenPadding

@Composable
internal fun JournalScreen(state: SettingsUiState, onBack: () -> Unit, onEvent: (SettingsEvent) -> Unit) {
    var confirmClear by rememberSaveable { mutableStateOf(false) }
    var showInfo by rememberSaveable { mutableStateOf(false) }
    val actions = buildList {
        add(ProductTopAction(PhosphorIcons.Regular.Info, "О журнале") { showInfo = true })
        if (state.diagnosticEvents.isNotEmpty()) {
            add(ProductTopAction(PhosphorIcons.Regular.Copy, "Скопировать журнал") {
                onEvent(SettingsEvent.CopyDiagnostics)
            })
            add(ProductTopAction(PhosphorIcons.Regular.Trash, "Очистить журнал") { confirmClear = true })
        }
    }
    ProductScaffold(title = "Диагностика", onBack = onBack, actions = actions) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                FeedCard {
                    Column(Modifier.padding(ProductTokens.Spacing.Section)) {
                        Text("Проверка распознавания", style = MaterialTheme.typography.titleMedium)
                        Text("Скорость и отчёт об устройстве. Без микрофона.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        TextButton(onClick = { onEvent(SettingsEvent.OpenBenchmark) }) {
                            Text("Проверить устройство")
                        }
                    }
                }
            }
            if (state.birdConnectionNeedsAttention) {
                item {
                    FeedCard {
                        Column(Modifier.padding(ProductTokens.Spacing.Section), verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                            Text("Птичка не подключена", style = MaterialTheme.typography.titleMedium)
                            Text("Разрешение включено, но Android не подключил службу Говоруна.",
                                style = MaterialTheme.typography.bodyMedium)
                            TextButton(onClick = { onEvent(SettingsEvent.OpenAccessibility) }) {
                                Text("Проверить в настройках Android")
                            }
                        }
                    }
                }
            }
            if (state.diagnosticEvents.isEmpty()) {
                item {
                    Column(Modifier.fillMaxWidth().padding(ProductTokens.Spacing.Section),
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                        Text("Пока нет событий", style = MaterialTheme.typography.bodyMedium)
                        Text("Здесь будут события работы Говоруна.",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
            state.diagnosticEvents.groupBy { it.day }.forEach { (day, events) ->
                item {
                    FeedCard {
                        Column {
                            FeedSectionHeader(day, horizontalPadding = ProductTokens.Spacing.Section)
                            events.forEachIndexed { index, event ->
                                if (index > 0) ProductDivider()
                                JournalEvent(event)
                            }
                        }
                    }
                }
            }
        }
    }
    if (showInfo) {
        AlertDialog(onDismissRequest = { showInfo = false }, title = { Text("О журнале") },
            text = { Text("Последние 200 событий хранятся только на этом устройстве. " +
                "Тексты диктовок и данные аккаунта в журнал не попадают.\n\n" +
                "Причины остановок появляются после следующего запуска, если их сообщает Android.") },
            confirmButton = { TextButton(onClick = { showInfo = false }) { Text("Понятно") } })
    }
    if (confirmClear) {
        AlertDialog(onDismissRequest = { confirmClear = false }, title = { Text("Очистить журнал?") },
            text = { Text("Записи будут удалены с этого устройства. Настройки и статистика сохранятся.") },
            confirmButton = { TextButton(onClick = {
                confirmClear = false
                onEvent(SettingsEvent.ClearDiagnostics)
            }) { Text("Очистить") } },
            dismissButton = { TextButton(onClick = { confirmClear = false }) { Text("Отмена") } })
    }
}

@Composable
private fun JournalEvent(event: DiagnosticEventRow) {
    var expanded by rememberSaveable(event) { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().heightIn(min = ProductTokens.Size.Row)
        .clickable(enabled = event.expandable) { expanded = !expanded }.padding(
        horizontal = ProductTokens.Spacing.Section, vertical = ProductTokens.Spacing.Md,
    ), horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
        Icon(when {
            event.attention -> PhosphorIcons.Regular.WarningCircle
            event.success -> PhosphorIcons.Regular.CheckCircle
            else -> PhosphorIcons.Regular.Info
        }, contentDescription = null, modifier = Modifier.size(ProductTokens.Size.SlotIcon), tint = when {
            event.attention -> MaterialTheme.colorScheme.error
            event.success -> MaterialTheme.colorScheme.primary
            else -> MaterialTheme.colorScheme.onSurfaceVariant
        })
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxs)) {
            Row(horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                Text(event.category, modifier = Modifier.weight(1f),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(event.time, style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                if (event.expandable) {
                    Icon(if (expanded) PhosphorIcons.Regular.CaretDown else PhosphorIcons.Regular.CaretRight,
                        contentDescription = if (expanded) "Свернуть" else "Развернуть",
                        modifier = Modifier.size(ProductTokens.Size.SlotIcon),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            Text(event.title, style = MaterialTheme.typography.bodyMedium)
            if (!expanded) event.compact?.let {
                Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            event.detail?.let {
                if (!event.expandable || expanded) {
                    it.lineSequence().forEach { line ->
                        val stage = line.length > 2 && line[0] in '1'..'3' && line[1] == '.'
                        Text(line.trimStart(), style = if (stage) MaterialTheme.typography.bodyMedium else MaterialTheme.typography.labelSmall,
                            modifier = Modifier.padding(top = if (stage) ProductTokens.Spacing.Sm else 0.dp),
                            color = if (stage) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }
}
