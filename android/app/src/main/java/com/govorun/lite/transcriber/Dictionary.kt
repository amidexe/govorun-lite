package com.govorun.lite.transcriber

import android.content.Context
import android.util.Log
import com.govorun.lite.shared.SharedDictionary
import com.govorun.lite.util.Prefs

/** Native preference port. All replacement rules execute in the shared core. */
object Dictionary {
    fun applyReplacements(context: Context, text: String): String = try {
        SharedDictionary.apply(text, Prefs.getDictionary(context), Prefs.isDictionaryEnabled(context))
    } catch (_: Exception) {
        Log.w("Dictionary", "Shared dictionary unavailable; preserving recognized text")
        text
    }

    internal fun applyRules(text: String, raw: String): String = SharedDictionary.apply(text, raw, true)
}
