package com.govorun.lite.model.settings

sealed interface SettingsChange {
    data class SetThemeMode(val value: UiThemeMode) : SettingsChange
    data class SetDynamicColor(val enabled: Boolean) : SettingsChange
    data class SetBubbleSide(val value: String) : SettingsChange
    data class SetBubbleSize(val value: Float) : SettingsChange
    data class SetBubbleAlpha(val value: Float) : SettingsChange
    data class SetBubbleRecordingAlpha(val value: Float) : SettingsChange
    data class SetBirdColor(val value: String) : SettingsChange
    data class SetEdgeMargin(val value: Int) : SettingsChange
    data class SetRecordingMode(val value: String) : SettingsChange
    data class SetHaptics(val enabled: Boolean) : SettingsChange
    data class SetPositionLocked(val enabled: Boolean) : SettingsChange
    data class SetPause(val value: String) : SettingsChange
    data class SetVoiceMarkerMode(val mode: VoiceMarkerMode) : SettingsChange
    data class SetHideInSearch(val enabled: Boolean) : SettingsChange
    data class SetKeepScreenOn(val enabled: Boolean) : SettingsChange
    data class SetRememberLastDictation(val enabled: Boolean) : SettingsChange
    data class SetAutoStop(val minutes: Int) : SettingsChange
}
