package com.govorun.lite.account

import android.content.Context

/** Device-local continuation marker; it carries no code, account or entitlement data. */
internal object StarterActivationHandoff {
    private const val PREFS = "govorun.account.starter_return"
    private const val KEY_CREATED_AT = "created_at"

    fun begin(context: Context, nowMillis: Long = System.currentTimeMillis()) {
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit().putLong(KEY_CREATED_AT, nowMillis).apply()
    }

    fun pending(context: Context, nowMillis: Long = System.currentTimeMillis()): Boolean {
        val createdAt = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getLong(KEY_CREATED_AT, 0L)
        return accountLoginReturnsToSubscription(createdAt, nowMillis)
    }

    fun clear(context: Context) {
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit().remove(KEY_CREATED_AT).apply()
    }
}
