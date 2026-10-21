package com.govorun.lite.settings

import android.content.Context
import org.json.JSONObject
import com.govorun.lite.model.settings.SettingsChange
import com.govorun.lite.settings.SettingsPreferences
import com.govorun.lite.util.HomePreferences
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import com.govorun.lite.util.UiAppearancePrefs
import com.govorun.lite.util.VoiceMarkerPrefs

internal class AndroidSettingsPreferences(context: Context) : SettingsPreferences {
    private val context = context.applicationContext

    override val stylizationEnabled: Boolean
        get() = Prefs.isStylizationEnabled(context)

    override fun disableStylization() = Prefs.setStylizationEnabled(context, false)

    override fun persist(change: SettingsChange) {
        when (change) {
            is SettingsChange.SetThemeMode -> UiAppearancePrefs.setThemeMode(context, change.value)
            is SettingsChange.SetDynamicColor -> UiAppearancePrefs.setDynamicColor(context, change.enabled)
            is SettingsChange.SetBubbleSide -> Prefs.setBubbleSide(context, change.value)
            is SettingsChange.SetBubbleSize -> Prefs.setBubbleSize(context, change.value)
            is SettingsChange.SetBubbleAlpha -> Prefs.setBubbleAlpha(context, change.value)
            is SettingsChange.SetBubbleRecordingAlpha -> Prefs.setBubbleRecordingAlpha(context, change.value)
            is SettingsChange.SetBirdColor -> Prefs.setBirdColor(context, change.value)
            is SettingsChange.SetEdgeMargin -> Prefs.setBubbleEdgeMargin(context, change.value)
            is SettingsChange.SetRecordingMode -> RecordingModePrefs.write(context, change.value)
            is SettingsChange.SetHaptics -> Prefs.setHapticsEnabled(context, change.enabled)
            is SettingsChange.SetPositionLocked -> Prefs.setBubblePositionLocked(context, change.enabled)
            is SettingsChange.SetPause -> Prefs.setPauseLength(context, change.value)
            is SettingsChange.SetVoiceMarkerMode -> VoiceMarkerPrefs.write(context, change.mode)
            is SettingsChange.SetHideInSearch -> Prefs.setHideInSearchEnabled(context, change.enabled)
            is SettingsChange.SetKeepScreenOn -> Prefs.setKeepScreenOnEnabled(context, change.enabled)
            is SettingsChange.SetRememberLastDictation ->
                com.govorun.lite.service.LastDictation.setEnabled(context, change.enabled)
            is SettingsChange.SetAutoStop -> Prefs.setAutoStopMinutes(context, change.minutes)
        }
    }

    override fun resetAppearance() {
        Prefs.setBubbleSize(context, Prefs.BUBBLE_SIZE_DEFAULT)
        Prefs.setBubbleSide(context, Prefs.BUBBLE_SIDE_RIGHT)
        Prefs.setBubbleAlpha(context, Prefs.BUBBLE_ALPHA_DEFAULT)
        Prefs.setBubbleRecordingAlpha(context, Prefs.BUBBLE_RECORDING_ALPHA_DEFAULT)
        Prefs.setBirdColor(context, com.govorun.lite.overlay.BirdColors.DEFAULT)
        Prefs.clearBubbleEdgeMargin(context)
    }

    override fun reset(defaults: JSONObject) {
        UiAppearancePrefs.reset(context)
        resetAppearance()
        RecordingModePrefs.write(context, defaults.getString("recordingMode"))
        Prefs.setHapticsEnabled(context, false)
        Prefs.setHideInSearchEnabled(context, false)
        Prefs.resetBubbleY(context)
        Prefs.setPauseLength(context, Prefs.PAUSE_DEFAULT)
        HomePreferences.setShowDictionaryOnHome(
            context,
            HomePreferences.DEFAULT_SHOW_DICTIONARY_ON_HOME,
        )
        Prefs.setKeepScreenOnEnabled(context, false)
        com.govorun.lite.service.LastDictation.setEnabled(context, false)
        Prefs.setAutoStopMinutes(context, Prefs.AUTO_STOP_DEFAULT)
        Prefs.setStylizationEnabled(context, defaults.getBoolean("stylizationEnabled"))
        Prefs.setStylizationStyle(context, defaults.getString("defaultStyle"))
        Prefs.setStylizationCustomInstruction(context, "")
    }
}
