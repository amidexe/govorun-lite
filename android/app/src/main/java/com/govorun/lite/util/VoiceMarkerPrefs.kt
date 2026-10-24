package com.govorun.lite.util

import com.govorun.lite.model.settings.VoiceMarkerMode
import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

object VoiceMarkerPrefs {
    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val MODE = "voice_marker_mode"

    fun read(context: Context): VoiceMarkerMode {
        val prefs = GovorunPreferences.of(context, FILE_NAME)
        return VoiceMarkerMode.fromStorageId(prefs.getString(MODE,
            if (prefs.getBoolean("voice_suffix_enabled", false)) VoiceMarkerMode.TEXT.storageId else null))
    }

    fun write(context: Context, mode: VoiceMarkerMode) {
        GovorunPreferences.of(context, FILE_NAME)
            .edit()
            .putString(MODE, mode.storageId)
            .apply()
    }

    internal fun deliveryMarker(context: Context): String {
        val prefs = GovorunPreferences.of(context, FILE_NAME)
        return if (!prefs.contains(MODE) && prefs.getBoolean("voice_suffix_enabled", false))
            "legacy_text" else read(context).name.lowercase()
    }
}
