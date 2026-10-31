package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

/**
 * «Кнопка громкости» (an experiment, off by default): a held volume-down dictates where the bubble is shown, and on the lock
 * screen records in the app given the volume-key permission. Off: Govorun never touches the key.
 */
internal object VolumeKeyPrefs {
    fun isEnabled(context: Context): Boolean =
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE).getBoolean(StorageKeys.Key.VOLUME_KEY_ENABLED, false)

    fun setEnabled(context: Context, enabled: Boolean) {
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE).edit().putBoolean(StorageKeys.Key.VOLUME_KEY_ENABLED, enabled).apply()
    }
}
