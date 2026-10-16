package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.settings.SettingsRules

/** Native storage adapter for the shared recording interaction policy. */
internal object RecordingModePrefs {
    const val AFTER_PAUSE = SettingsRules.AFTER_PAUSE
    const val REPEAT_TAP = SettingsRules.REPEAT_TAP

    fun defaultMode(): String = SettingsRules.recording("read", null).getString("mode")

    private const val KEY = "recording_interaction_mode_v2"

    fun onboardingChoice(context: Context): String = SettingsRules.recording("onboarding",
        context.getSharedPreferences(StorageKeys.PREFS_FILE, Context.MODE_PRIVATE)
            .getString(KEY, null),
    ).getString("mode")

    fun read(context: Context): String {
        val raw = context.getSharedPreferences(
            StorageKeys.PREFS_FILE,
            Context.MODE_PRIVATE,
        ).getString(KEY, null)
        return SettingsRules.recording("read", raw, Prefs.isStylizationEnabled(context)).getString("mode")
    }

    fun write(context: Context, value: String) {
        val validated = SettingsRules.recording("select", value).getString("mode")
        context.getSharedPreferences(
            StorageKeys.PREFS_FILE,
            Context.MODE_PRIVATE,
        ).edit().putString(KEY, validated).apply()
    }

    fun enableStylizationMode(context: Context) {
        write(context, REPEAT_TAP)
    }

}
