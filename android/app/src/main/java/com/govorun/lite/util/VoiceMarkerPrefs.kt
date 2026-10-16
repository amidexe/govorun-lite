package com.govorun.lite.util

import com.govorun.lite.model.settings.VoiceMarkerMode
import android.content.Context

object VoiceMarkerPrefs {
    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val MODE = "voice_marker_mode"

    fun read(context: Context): VoiceMarkerMode {
        val prefs = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
        return VoiceMarkerMode.fromStorageId(prefs.getString(MODE,
            if (prefs.getBoolean("voice_suffix_enabled", false)) VoiceMarkerMode.TEXT.storageId else null))
    }

    fun write(context: Context, mode: VoiceMarkerMode) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(MODE, mode.storageId)
            .apply()
    }

    internal fun deliveryMarker(context: Context): String {
        val prefs = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
        return if (!prefs.contains(MODE) && prefs.getBoolean("voice_suffix_enabled", false))
            "legacy_text" else read(context).name.lowercase()
    }
}
