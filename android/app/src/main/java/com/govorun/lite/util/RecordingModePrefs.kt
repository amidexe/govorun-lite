package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.settings.SettingsRules
import com.govorun.lite.storage.GovorunPreferences

/** Native storage adapter for the shared recording interaction policy. */
internal object RecordingModePrefs {
    const val AFTER_PAUSE = SettingsRules.AFTER_PAUSE
    const val REPEAT_TAP = SettingsRules.REPEAT_TAP

    fun defaultMode(): String = SettingsRules.recording("read", null).getString("mode")

    private const val KEY = "recording_interaction_mode_v2"

    fun onboardingChoice(context: Context): String = SettingsRules.recording("onboarding",
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE)
            .getString(KEY, null),
    ).getString("mode")

    fun read(context: Context): String {
        val raw = GovorunPreferences.of(context, StorageKeys.PREFS_FILE).getString(KEY, null)
        return SettingsRules.recording("read", raw, Prefs.isStylizationEnabled(context)).getString("mode")
    }

    fun write(context: Context, value: String) {
        val validated = SettingsRules.recording("select", value).getString("mode")
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE).edit().putString(KEY, validated).apply()
    }

    fun enableStylizationMode(context: Context) {
        write(context, REPEAT_TAP)
    }

}
