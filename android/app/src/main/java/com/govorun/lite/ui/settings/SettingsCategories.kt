package com.govorun.lite.ui.settings

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.govorun.lite.BuildConfig
import com.govorun.lite.shared.SharedDictationDelivery
import com.govorun.lite.shared.SharedDictationRules
import com.govorun.lite.model.settings.SettingsChange
import com.govorun.lite.model.settings.UiThemeMode
import com.govorun.lite.model.settings.VoiceMarkerMode
import com.govorun.lite.ui.AppBuildIdentity
import com.govorun.lite.util.Prefs

private data class SettingChoice<T>(
    val value: T,
    val label: String,
)

@Composable
internal fun DictationSettings(
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    ServiceControlSettings(state.serviceEnabled, onEvent)
    SettingsSection("Распознавание") {
        ChoiceValueRow(
            title = "Пауза после речи",
            summary = "Длинная пауза помогает говорить медленнее, не разрывая фразу",
            selected = state.pause,
            choices = listOf(
                SettingChoice(Prefs.PAUSE_SHORT, "Короткая"),
                SettingChoice(Prefs.PAUSE_MEDIUM, "Средняя"),
                SettingChoice(Prefs.PAUSE_LONG, "Длинная"),
            ),
            onSelected = { onEvent(SettingsEvent.Change(SettingsChange.SetPause(it))) },
        )
    }
    SettingsSection("Длительная запись") {
        SettingsSwitchRow(
            title = "Не блокировать экран",
            summary = if (state.keepScreenOn) {
                "Пока Говорун записывает речь"
            } else {
                "Android блокирует экран как обычно"
            },
            checked = state.keepScreenOn,
            onCheckedChange = { onEvent(SettingsEvent.Change(SettingsChange.SetKeepScreenOn(it))) },
        )
        if (state.keepScreenOn) {
            SettingsDivider()
            ChoiceValueRow(
                title = "Остановить запись через",
                selected = state.autoStopMinutes,
                choices = listOf(
                    SettingChoice(Prefs.AUTO_STOP_15M, "15 минут"),
                    SettingChoice(Prefs.AUTO_STOP_1H, "1 час"),
                    SettingChoice(Prefs.AUTO_STOP_3H, "3 часа"),
                    SettingChoice(Prefs.AUTO_STOP_OFF, "Без ограничения"),
                ),
                onSelected = { onEvent(SettingsEvent.Change(SettingsChange.SetAutoStop(it))) },
            )
        }
    }

}

@Composable
internal fun AfterRecordingSettings(
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    processingSettingsSections.forEach { section ->
        SettingsSection(section.title) {
            section.entries.forEachIndexed { index, entry ->
                if (index > 0) SettingsDivider()
                ProcessingSettingsEntryRow(entry.id, state, onEvent)
            }
        }
    }
}

@Composable
private fun ProcessingSettingsEntryRow(
    entry: ProcessingSettingsEntryId,
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    when (entry) {
        ProcessingSettingsEntryId.STYLE_AND_FORMATTING -> SettingsLinkRow(
            title = "Стиль и оформление",
            summary = if (state.selfHosted) "Не применяется, пока включён свой сервер" else state.stylizationSummary,
            enabled = !state.selfHosted,
            onClick = { onEvent(SettingsEvent.OpenStylization) },
        )
        ProcessingSettingsEntryId.ACCESS_AND_LIMITS -> SettingsLinkRow(
            title = "Подписка и лимиты",
            summary = state.accessSummary,
            onClick = { onEvent(SettingsEvent.OpenSubscription) },
        )
        ProcessingSettingsEntryId.DICTIONARY_AND_REPLACEMENTS -> SettingsLinkRow(
            title = "Словарь и автозамены",
            summary = "Исправлять имена, термины и другие указанные слова",
            onClick = { onEvent(SettingsEvent.OpenDictionary) },
        )
        ProcessingSettingsEntryId.VOICE_TEXT_MARKER -> ChoiceValueRow(
            title = "Помечать голосовой текст",
            summary = if (state.legacyVoiceMarker)
                "Дописывает «[текст распознан с голоса]» на новой строке после каждой диктовки."
                else voiceMarkerSummary(state.voiceMarkerMode),
            selected = state.voiceMarkerMode,
            choices = listOf(
                SettingChoice(VoiceMarkerMode.OFF, "Не помечать"),
                SettingChoice(VoiceMarkerMode.MICROPHONE, "Краткий — 🔊 в начале"),
                SettingChoice(VoiceMarkerMode.TEXT, "Подробный — строка в конце"),
            ),
            onSelected = { onEvent(SettingsEvent.Change(SettingsChange.SetVoiceMarkerMode(it))) },
        )
        ProcessingSettingsEntryId.SELF_HOSTED_SERVER -> SettingsLinkRow(
            title = "Свой сервер",
            summary = "Отдельная онлайн-обработка; подписка Говоруна не используется",
            onClick = { onEvent(SettingsEvent.OpenSelfHostedServer) },
        )
    }
}

@Composable
internal fun ApplicationSettings(
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    SettingsSection("Оформление") {
        ChoiceValueRow(
            title = "Тема приложения",
            selected = state.themeMode,
            choices = listOf(
                SettingChoice(UiThemeMode.SYSTEM, "Как в системе"),
                SettingChoice(UiThemeMode.LIGHT, "Светлая"),
                SettingChoice(UiThemeMode.DARK, "Тёмная"),
            ),
            onSelected = { onEvent(SettingsEvent.Change(SettingsChange.SetThemeMode(it))) },
        )
        SettingsDivider()
        SettingsSwitchRow(
            title = "Использовать цвета Android",
            summary = "Подстраивать палитру под обои устройства",
            checked = state.dynamicColor,
            onCheckedChange = { onEvent(SettingsEvent.Change(SettingsChange.SetDynamicColor(it))) },
        )
    }
    SettingsSection("Другие приложения") {
        SettingsLinkRow(
            title = "API для приложений",
            summary = "Локальное распознавание для диктофонов, заметок и других приложений",
            onClick = { onEvent(SettingsEvent.OpenAuthorizedApps) },
        )
    }
    SettingsSection("О приложении") {
        SettingsLinkRow(
            title = "Диагностика",
            summary = "События работы и причины неполадок",
            onClick = { onEvent(SettingsEvent.OpenJournal) },
        )
        SettingsDivider()
        SettingsSummaryRow(
            title = "Версия Говоруна",
            value = AppBuildIdentity.compactValue(
                BuildConfig.VERSION_NAME,
                BuildConfig.VERSION_CODE,
                BuildConfig.BUILD_ID,
            ),
        )
        SettingsDivider()
        SettingsLinkRow(
            title = "Восстановить настройки",
            summary = "Вернуть оформление и диктовку к исходным значениям",
            onClick = { onEvent(SettingsEvent.Reset) },
        )
    }
    SettingsSection("Данные") {
        SettingsLinkRow(
            title = "Сбросить статистику",
            summary = "Удалить счётчики слов и времени речи",
            onClick = { onEvent(SettingsEvent.ResetStatistics) },
        )
    }
}

@Composable
private fun <T> ChoiceValueRow(
    title: String,
    summary: String? = null,
    selected: T,
    choices: List<SettingChoice<T>>,
    onSelected: (T) -> Unit,
) {
    var sheetOpen by remember { mutableStateOf(false) }
    val current = choices.firstOrNull { it.value == selected } ?: choices.last()

    SettingsLinkRow(
        title = title,
        summary = summary,
        value = current.label,
        onClick = { sheetOpen = true },
    )
    if (sheetOpen) {
        SettingsChoiceSheet(
            title = title,
            selected = current,
            options = choices,
            optionLabel = SettingChoice<T>::label,
            onSelected = { onSelected(it.value) },
            onDismissRequest = { sheetOpen = false },
        )
    }
}

private fun voiceMarkerSummary(mode: VoiceMarkerMode): String {
    val minimum = SharedDictationRules.minimumCharacters
    return when (mode) {
        VoiceMarkerMode.OFF ->
            "Показывает собеседнику, что длинное сообщение продиктовано · от $minimum символов"
        VoiceMarkerMode.MICROPHONE ->
            "🔊 добавляется перед текстом · только от $minimum символов"
        VoiceMarkerMode.TEXT ->
            "В конце отдельная строка «${SharedDictationDelivery.signature}» · только от $minimum символов"
    }
}
