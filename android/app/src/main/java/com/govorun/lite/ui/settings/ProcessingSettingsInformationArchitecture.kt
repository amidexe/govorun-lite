package com.govorun.lite.ui.settings

internal enum class ProcessingSettingsEntryId {
    STYLE_AND_FORMATTING,
    ACCESS_AND_LIMITS,
    DICTIONARY_AND_REPLACEMENTS,
    VOICE_TEXT_MARKER,
    SELF_HOSTED_SERVER,
}

internal enum class ProcessingRoute {
    OFFICIAL_SERVICE,
    ON_DEVICE,
    SELF_HOSTED_ONLINE,
}

internal data class ProcessingSettingsEntry(
    val id: ProcessingSettingsEntryId,
    val route: ProcessingRoute,
)

internal data class ProcessingSettingsSection(
    val title: String,
    val entries: List<ProcessingSettingsEntry>,
)

internal val processingSettingsSections = listOf(
    ProcessingSettingsSection(
        title = "Сервис Говоруна",
        entries = listOf(
            ProcessingSettingsEntry(
                ProcessingSettingsEntryId.STYLE_AND_FORMATTING,
                ProcessingRoute.OFFICIAL_SERVICE,
            ),
            ProcessingSettingsEntry(
                ProcessingSettingsEntryId.ACCESS_AND_LIMITS,
                ProcessingRoute.OFFICIAL_SERVICE,
            ),
        ),
    ),
    ProcessingSettingsSection(
        title = "Возможности без подписки",
        entries = listOf(
            ProcessingSettingsEntry(
                ProcessingSettingsEntryId.DICTIONARY_AND_REPLACEMENTS,
                ProcessingRoute.ON_DEVICE,
            ),
            ProcessingSettingsEntry(
                ProcessingSettingsEntryId.VOICE_TEXT_MARKER,
                ProcessingRoute.ON_DEVICE,
            ),
            ProcessingSettingsEntry(
                ProcessingSettingsEntryId.SELF_HOSTED_SERVER,
                ProcessingRoute.SELF_HOSTED_ONLINE,
            ),
        ),
    ),
)
