package com.govorun.lite.account

import android.content.Context
import java.security.MessageDigest
import org.json.JSONObject

enum class AccountDataSyncMode {
    EVALUATING,
    PENDING,
    ENABLED,
    LOCAL_ONLY,
}

internal class AccountSyncEnrollmentStore(context: Context) {
    private val preferences = context.applicationContext.getSharedPreferences(
        FILE_NAME,
        Context.MODE_PRIVATE,
    )

    @Synchronized
    fun resolve(
        accountId: String,
        localContent: SyncedContentDocument,
        hasExistingSyncHistory: Boolean,
        hasRemoteData: Boolean? = null,
        statistics: JSONObject,
    ): AccountDataSyncMode {
        val resolved = resolveAccountDataSyncMode(
            stored(accountId),
            localContent,
            hasExistingSyncHistory,
            hasOtherAccountHistory = preferences.all.keys.any {
                it.startsWith("mode_") && it != key(accountId)
            },
            explicitDecisionRequested = preferences.getBoolean("requested_" + key(accountId), false),
            hasRemoteData = hasRemoteData,
            statistics = statistics,
        )
        save(accountId, resolved)
        return resolved
    }

    fun mode(accountId: String): AccountDataSyncMode =
        stored(accountId) ?: AccountDataSyncMode.EVALUATING

    fun enable(accountId: String) = save(accountId, AccountDataSyncMode.ENABLED)

    fun keepLocal(accountId: String) = save(accountId, AccountDataSyncMode.LOCAL_ONLY)

    fun requestDecision(accountId: String) {
        preferences.edit().putBoolean("requested_" + key(accountId), true).commit()
        save(accountId, AccountDataSyncMode.PENDING)
    }

    fun isEnabled(accountId: String): Boolean = mode(accountId) == AccountDataSyncMode.ENABLED

    private fun stored(accountId: String): AccountDataSyncMode? = preferences
        .getString(key(accountId), null)
        ?.let { raw -> AccountDataSyncMode.entries.firstOrNull { it.name == raw } }
        ?.takeUnless { it == AccountDataSyncMode.EVALUATING }

    private fun save(accountId: String, mode: AccountDataSyncMode) {
        require(mode != AccountDataSyncMode.EVALUATING)
        preferences.edit().putString(key(accountId), mode.name).apply {
            if (mode != AccountDataSyncMode.PENDING) remove("requested_" + key(accountId))
        }.commit()
    }

    private fun key(accountId: String): String = "mode_" + MessageDigest.getInstance("SHA-256")
        .digest(accountId.toByteArray(Charsets.UTF_8))
        .take(12)
        .joinToString("") { "%02x".format(it) }

    private companion object {
        const val FILE_NAME = "govorun_account_sync_enrollment_v1"
    }
}

internal fun resolveAccountDataSyncMode(
    stored: AccountDataSyncMode?,
    localContent: SyncedContentDocument,
    hasExistingSyncHistory: Boolean,
    hasOtherAccountHistory: Boolean = false,
    explicitDecisionRequested: Boolean = false,
    hasRemoteData: Boolean? = null,
    statistics: JSONObject,
): AccountDataSyncMode {
    fun AccountDataSyncMode.wire() = when (this) {
        AccountDataSyncMode.EVALUATING -> "evaluating"
        AccountDataSyncMode.PENDING -> "pending"
        AccountDataSyncMode.ENABLED -> "enabled"
        AccountDataSyncMode.LOCAL_ONLY -> "localOnly"
    }
    val result = SynchronizationRules.evaluate(JSONObject().put("kind", "enrollment")
        .put("stored", stored?.wire() ?: JSONObject.NULL).put("content", SyncDocumentJson.encode(localContent))
        .put("statistics", statistics).put("history", hasExistingSyncHistory)
        .put("otherAccountHistory", hasOtherAccountHistory).put("requested", explicitDecisionRequested)
        .put("remoteData", hasRemoteData ?: JSONObject.NULL))
    return AccountDataSyncMode.entries.first { it.wire() == result.getString("mode") }
}
