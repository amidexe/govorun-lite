package com.govorun.lite.service

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences
import com.govorun.lite.util.StorageKeys

/**
 * «Помнить последнюю диктовку»: off by default. When the person turns it on, the last dictated
 * text is kept only in this process's memory — never on disk — so a dictation that could not be
 * inserted can be copied from the home screen. Replaced by the next dictation, dropped when the
 * option is turned off, gone when the process ends. Same rule as the desktop tray item.
 */
internal object LastDictation {
    @Volatile private var text: String? = null
    // The local transcript, when online improvement changed it: both stay side by side for a comparison.
    @Volatile private var original: String? = null

    // Only the choice is stored; the text never is.
    fun isEnabled(context: Context): Boolean =
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE)
            .getBoolean(StorageKeys.Key.REMEMBER_LAST_DICTATION, false)

    fun setEnabled(context: Context, enabled: Boolean) {
        GovorunPreferences.of(context, StorageKeys.PREFS_FILE)
            .edit()
            .putBoolean(StorageKeys.Key.REMEMBER_LAST_DICTATION, enabled)
            .apply()
        if (!enabled) { text = null; original = null }
    }

    fun remember(context: Context, value: String) {
        if (!isEnabled(context)) {
            text = null
            original = null
            return
        }
        value.trim().takeIf { it.isNotEmpty() }?.let { text = it; original = null }
    }

    /** Called right after the improved text was remembered; an unchanged transcript adds nothing. */
    fun attachOriginal(context: Context, value: String) {
        if (!isEnabled(context)) return
        original = value.trim().takeIf { it.isNotEmpty() && it != text }
    }

    fun peekOriginal(context: Context): String? = original.takeIf { isEnabled(context) && text != null }

    fun peek(context: Context): String? = text.takeIf { isEnabled(context) }
}
