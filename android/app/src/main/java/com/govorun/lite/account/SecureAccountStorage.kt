package com.govorun.lite.account

import android.content.Context
import java.util.UUID
import org.json.JSONObject

/** Protected device identity and anonymous reserve I/O. Account credentials,
 * OAuth attempts and checkout journals are owned by SharedAccountStorage.
 */
internal class SecureAccountStorage(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private val cipher = AccountStorageCipher(KEY_ALIAS)

    @Synchronized
    fun installationId(): String = when (val read = readSecureRecord(
        { prefs.getString(KEY_INSTALLATION, null) }, ::decrypt,
        { raw -> JSONObject(raw).getString("id").also { id ->
            require(UUID.fromString(id).toString().equals(id, ignoreCase = true))
        } },
    )) {
        is SecureRecordRead.Available -> read.value
        is SecureRecordRead.Unavailable -> throw SecureStorageUnavailableException()
        SecureRecordRead.Missing -> UUID.randomUUID().toString().also { value ->
            writeJson(KEY_INSTALLATION, JSONObject().put("id", value))
        }
    }

    private fun writeJson(key: String, json: JSONObject) {
        if (!prefs.edit().putString(key, encrypt(json.toString())).commit()) {
            throw SecureStorageUnavailableException()
        }
    }

    private fun encrypt(value: String): String = cipher.encrypt(value)
    private fun decrypt(value: String): String = cipher.decryptPreviousUserRecord(value)

    private companion object {
        const val PREFS = "govorun_account_secure_v1"
        const val KEY_ALIAS = "govorun_account_session_v1"
        const val KEY_INSTALLATION = "installation"
    }
}
