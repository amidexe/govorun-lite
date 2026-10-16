package com.govorun.lite.ui.settings

import androidx.activity.compose.BackHandler
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.vectorResource
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.GearSix
import com.adamglin.phosphoricons.regular.MagicWand
import com.adamglin.phosphoricons.regular.Microphone
import com.govorun.lite.R
import com.product.ui.ProductSectionItem
import com.product.ui.ProductSettingsShell

private enum class SettingsTab(
    val title: String,
    val icon: ImageVector?,
) {
    DICTATION("Диктовка", PhosphorIcons.Regular.Microphone),
    BIRD("Птичка", null),
    AFTER_RECORDING("Обработка", PhosphorIcons.Regular.MagicWand),
    APPLICATION("Приложение", PhosphorIcons.Regular.GearSix),
}

@Composable
fun SettingsScreen(
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    var selectedName by rememberSaveable { mutableStateOf(SettingsTab.DICTATION.name) }
    var journalOpen by rememberSaveable { mutableStateOf(false) }
    var confirmReset by rememberSaveable { mutableStateOf(false) }
    var confirmStatisticsReset by rememberSaveable { mutableStateOf(false) }
    val available = SettingsTab.entries
    val selected = available.firstOrNull { it.name == selectedName }
        ?: SettingsTab.DICTATION
    val tabs = available.map { tab ->
        ProductSectionItem(
            key = tab,
            title = tab.title,
            icon = tab.icon ?: ImageVector.vectorResource(R.drawable.ic_bird_24),
        )
    }

    if (journalOpen) {
        BackHandler { journalOpen = false }
        JournalScreen(state, onBack = { journalOpen = false }, onEvent = onEvent)
        return
    }

    ProductSettingsShell(
        title = "Настройки",
        items = tabs,
        selected = selected,
        onSelected = { selectedName = it.name },
        onBack = { onEvent(SettingsEvent.Back) },
    ) {
        when (selected) {
            SettingsTab.DICTATION -> item { DictationSettings(state, onEvent) }
            SettingsTab.BIRD -> item { AppearanceSettings(state, onEvent) }
            SettingsTab.AFTER_RECORDING -> item { AfterRecordingSettings(state, onEvent) }
            SettingsTab.APPLICATION -> item {
                ApplicationSettings(state) {
                    when (it) {
                        SettingsEvent.Reset -> confirmReset = true
                        SettingsEvent.ResetStatistics -> confirmStatisticsReset = true
                        SettingsEvent.OpenJournal -> journalOpen = true
                        else -> onEvent(it)
                    }
                }
            }
        }
    }
    if (confirmReset) {
        AlertDialog(
            onDismissRequest = { confirmReset = false },
            title = { Text("Восстановить настройки?") },
            text = {
                Text(
                    "Оформление и параметры диктовки вернутся к исходным. " +
                        "Словарь и статистика сохранятся.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmReset = false
                        onEvent(SettingsEvent.Reset)
                    },
                ) { Text("Восстановить") }
            },
            dismissButton = {
                TextButton(onClick = { confirmReset = false }) { Text("Отмена") }
            },
        )
    }
    if (confirmStatisticsReset) {
        AlertDialog(
            onDismissRequest = { confirmStatisticsReset = false },
            title = { Text("Сбросить статистику?") },
            text = {
                Text(
                    "Счётчики слов и времени речи будут удалены. " +
                        "Это действие нельзя отменить.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmStatisticsReset = false
                        onEvent(SettingsEvent.ResetStatistics)
                    },
                ) { Text("Сбросить") }
            },
            dismissButton = {
                TextButton(onClick = { confirmStatisticsReset = false }) { Text("Отмена") }
            },
        )
    }
}
