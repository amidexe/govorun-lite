package com.govorun.lite.util

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

internal object AppFilterPrefs {
    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val KEY_MODE = StorageKeys.Key.APP_FILTER_MODE
    private const val KEY_PACKAGES = StorageKeys.Key.APP_FILTER_PACKAGES

    fun mode(context: Context): String =
        prefs(context).getString(KEY_MODE, Prefs.APP_FILTER_BLACKLIST)
            ?: Prefs.APP_FILTER_BLACKLIST

    fun setMode(context: Context, mode: String) {
        prefs(context).edit().putString(KEY_MODE, mode).apply()
    }

    fun packages(context: Context): Set<String> =
        prefs(context).getStringSet(KEY_PACKAGES, emptySet())?.toSet().orEmpty()

    fun setPackages(context: Context, packages: Set<String>) {
        prefs(context).edit().putStringSet(KEY_PACKAGES, packages.toSet()).apply()
    }

    fun isFiltered(context: Context, packageName: String): Boolean {
        val selected = packages(context)
        return when (mode(context)) {
            Prefs.APP_FILTER_WHITELIST -> selected.isNotEmpty() && packageName !in selected
            else -> packageName in selected
        }
    }

    private fun prefs(context: Context) =
        GovorunPreferences.of(context, FILE_NAME)
}
