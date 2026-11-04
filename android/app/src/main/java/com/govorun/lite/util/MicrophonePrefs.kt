package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

/** Missing preference follows connected headsets; a saved phone choice is never overridden. */
internal object MicrophonePrefs {
    fun headset(context: Context): Boolean =
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE).getBoolean(StorageKeys.Key.HEADSET_MICROPHONE, true)

    fun setHeadset(context: Context, enabled: Boolean) {
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE).edit()
            .putBoolean(StorageKeys.Key.HEADSET_MICROPHONE, enabled).apply()
    }
}
