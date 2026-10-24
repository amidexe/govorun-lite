package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.storage.GovorunPreferences

internal object AccountLoginReturnHandoff {
    private const val PREFS = "govorun.account.login_return"
    private const val KEY_CREATED_AT = "subscription_created_at"
    private const val KEY_STYLIZATION_AT = "stylization_created_at"

    fun beginSubscription(context: Context, nowMillis: Long = System.currentTimeMillis()) {
        GovorunPreferences.of(context.applicationContext, PREFS)
            .edit().remove(KEY_STYLIZATION_AT).putLong(KEY_CREATED_AT, nowMillis).apply()
    }

    fun beginStylization(context: Context, nowMillis: Long = System.currentTimeMillis()) {
        GovorunPreferences.of(context.applicationContext, PREFS)
            .edit().remove(KEY_CREATED_AT).putLong(KEY_STYLIZATION_AT, nowMillis).apply()
    }

    fun clear(context: Context) {
        GovorunPreferences.of(context.applicationContext, PREFS)
            .edit().remove(KEY_CREATED_AT).remove(KEY_STYLIZATION_AT).apply()
    }

    fun consume(
        context: Context,
        nowMillis: Long = System.currentTimeMillis(),
    ): AccountLoginDestination {
        val prefs = GovorunPreferences.of(context.applicationContext, PREFS)
        val createdAt = prefs.getLong(KEY_CREATED_AT, 0L)
        val stylizationAt = prefs.getLong(KEY_STYLIZATION_AT, 0L)
        clear(context)
        return accountLoginDestination(createdAt, stylizationAt, nowMillis)
    }
}

internal enum class AccountLoginDestination { HOME, SUBSCRIPTION, STYLIZATION }

internal fun accountLoginDestination(subscriptionAt: Long, stylizationAt: Long, nowMillis: Long): AccountLoginDestination =
    AccountLoginDestination.valueOf(com.govorun.lite.shared.SharedBusiness.evaluate(org.json.JSONObject()
        .put("operation", "authHandoff").put("subscriptionAt", subscriptionAt)
        .put("stylizationAt", stylizationAt).put("now", nowMillis)).getString("destination"))

internal fun accountLoginReturnsToSubscription(createdAtMillis: Long, nowMillis: Long): Boolean =
    accountLoginDestination(createdAtMillis, 0, nowMillis) == AccountLoginDestination.SUBSCRIPTION
