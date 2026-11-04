package com.govorun.lite.ui

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

/** Local discovery state, unrelated to accounts, entitlement or server announcements. */
internal object ProductNotice {
    const val REVISION = "echo-2026-11"
    private const val FILE = "govorun_product_notices"
    private const val KEY = "seen_revision"
    fun unseen(context: Context): Boolean =
        GovorunPreferences.of(context, FILE).getString(KEY, null) != REVISION
    fun viewed(context: Context) {
        GovorunPreferences.of(context, FILE).edit().putString(KEY, REVISION).apply()
    }
}
