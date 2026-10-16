package com.govorun.lite.util

import android.content.Context

/** Device-local choices that only affect the Android home screen. */
object HomePreferences {
    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val KEY_SHOW_DICTIONARY_ON_HOME = "show_dictionary_on_home"
    private const val KEY_STARTER_GUIDANCE_DISMISSED = "starter_guidance_dismissed"

    const val DEFAULT_SHOW_DICTIONARY_ON_HOME = false

    fun showDictionaryOnHome(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(
                KEY_SHOW_DICTIONARY_ON_HOME,
                DEFAULT_SHOW_DICTIONARY_ON_HOME,
            )

    fun setShowDictionaryOnHome(context: Context, visible: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_SHOW_DICTIONARY_ON_HOME, visible)
            .apply()
    }

    fun starterGuidanceDismissed(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_STARTER_GUIDANCE_DISMISSED, false)

    fun dismissStarterGuidance(context: Context) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit().putBoolean(KEY_STARTER_GUIDANCE_DISMISSED, true).apply()
    }

    fun resetStarterGuidance(context: Context) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit().remove(KEY_STARTER_GUIDANCE_DISMISSED).apply()
    }
}
