package com.govorun.lite.ui.settings

import com.govorun.lite.model.settings.SettingsChange
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import com.govorun.lite.model.settings.UiThemeMode
import com.govorun.lite.model.settings.VoiceMarkerMode

data class SettingsUiState(
    val selfHosted: Boolean = false,
    val themeMode: UiThemeMode,
    val dynamicColor: Boolean,
    val bubbleSide: String,
    val bubbleSize: Float,
    val bubbleAlpha: Float,
    val edgeMargin: Int,
    val recordingMode: String,
    val haptics: Boolean,
    val positionLocked: Boolean,
    val pause: String,
    val stylizationSummary: String,
    val accessSummary: String,
    val voiceMarkerMode: VoiceMarkerMode,
    val hideInSearch: Boolean,
    val appFilterSummary: String,
    val keepScreenOn: Boolean,
    val autoStopMinutes: Int,
    val serviceEnabled: Boolean,
    val diagnosticEvents: List<DiagnosticEventRow> = emptyList(),
    val birdConnectionNeedsAttention: Boolean = false,
    val edgeMarginCustomized: Boolean = false,
    val recordingAlpha: Float = Prefs.BUBBLE_RECORDING_ALPHA_DEFAULT,
    val birdColor: String = com.govorun.lite.overlay.BirdColors.DEFAULT,
    val edgeMarginMax: Int = Prefs.BUBBLE_EDGE_MARGIN_MAX,
    val legacyVoiceMarker: Boolean = false,
    val rememberLastDictation: Boolean = false,
    val noNetworkInRoaming: Boolean = true,
    /** The documents base did not open: settings read as defaults and are not kept. */
    val storageUnavailable: Boolean = false,
) {
    companion object {
        fun loading() = SettingsUiState(
            themeMode = UiThemeMode.SYSTEM,
            dynamicColor = false,
            bubbleSide = Prefs.BUBBLE_SIDE_RIGHT,
            bubbleSize = Prefs.BUBBLE_SIZE_DEFAULT,
            bubbleAlpha = Prefs.BUBBLE_ALPHA_DEFAULT,
            edgeMargin = Prefs.BUBBLE_EDGE_MARGIN_DEFAULT,
            recordingMode = RecordingModePrefs.defaultMode(),
            haptics = false,
            positionLocked = false,
            pause = Prefs.PAUSE_DEFAULT,
            stylizationSummary = "",
            accessSummary = "",
            voiceMarkerMode = VoiceMarkerMode.OFF,
            hideInSearch = false,
            appFilterSummary = "",
            keepScreenOn = false,
            autoStopMinutes = Prefs.AUTO_STOP_DEFAULT,
            serviceEnabled = false,
        )
    }
}

data class DiagnosticEventRow(
    val day: String,
    val time: String,
    val title: String,
    val detail: String? = null,
    val attention: Boolean = false,
    val category: String = "Приложение",
    val success: Boolean = false,
    val expandable: Boolean = false,
    val compact: String? = null,
)

sealed interface SettingsEvent {
    data object Back : SettingsEvent
    data class Change(val value: SettingsChange) : SettingsEvent
    data object OpenStylization : SettingsEvent
    data object OpenSelfHostedServer : SettingsEvent
    data object OpenSubscription : SettingsEvent
    data object OpenDictionary : SettingsEvent
    data object OpenAppFilter : SettingsEvent
    data object OpenBenchmark : SettingsEvent
    data object OpenAuthorizedApps : SettingsEvent
    data object OpenAccessibility : SettingsEvent
    data object OpenJournal : SettingsEvent
    data object ConfirmDisableService : SettingsEvent
    data object ClearDiagnostics : SettingsEvent
    data object CopyDiagnostics : SettingsEvent
    data object ResetStatistics : SettingsEvent
    data object Reset : SettingsEvent
    data object ResetAppearance : SettingsEvent

    val updatesSnapshot: Boolean
        get() = this !is Back &&
            this !is OpenStylization &&
            this !is OpenSelfHostedServer &&
            this !is OpenSubscription &&
            this !is OpenDictionary &&
            this !is OpenAppFilter &&
            this !is OpenBenchmark &&
            this !is OpenAuthorizedApps &&
            this !is OpenAccessibility &&
            this !is OpenJournal &&
            this !is CopyDiagnostics &&
            this !is ConfirmDisableService
}
