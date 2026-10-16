package com.govorun.lite.account

import android.content.Context
import android.content.SharedPreferences
import java.util.UUID
import org.json.JSONObject

/** Protected I/O for AccountSession/AuthFlow. No account or UI side effects.
 * Existing session credentials keep their preference identity and Keystore key.
 * Explicit recovery selects a fresh namespace without decrypting the old one.
 */
internal class SharedAccountStorage(
    private val prefs: SharedPreferences,
    private val keyAlias: String = "govorun_account_session_v1",
    private val cipher: (String) -> AccountRecordCipher = ::AccountStorageCipher,
) {
    constructor(context: Context) : this(context.applicationContext.getSharedPreferences(
        "govorun_account_secure_v1", Context.MODE_PRIVATE))

    @Synchronized
    fun execute(command: JSONObject): JSONObject = try {
        require(command.getString("operation") == "storage")
        when (command.getString("action")) {
            "load" -> load("session", "session", "ended") {
                if (it.optInt("version") != 1 || !it.has("tokens")) throw InvalidAccountRecord()
                it
            }
            "save" -> {
                val value = command.getJSONObject("session")
                require(value.getInt("version") == 1 && value.has("tokens"))
                save("session", value, "ended")
                available()
            }
            "end" -> { fence("session", "ended"); available() }
            "restart" -> { restart(); available() }
            "authLoad" -> load("login_attempt", "attempt", "authCancelled") { it }
            "authSave" -> {
                require(command.has("attempt"))
                if (command.isNull("attempt")) fence("login_attempt", "authCancelled")
                else save("login_attempt", command.getJSONObject("attempt"), "authCancelled")
                available()
            }
            "checkoutLoad" -> {
                val slot = checkoutSlot(command)
                val value = read(slot, scoped = false)
                if (value == null) missing() else {
                    if (!value.has("record") || value.toString().toByteArray(Charsets.UTF_8).size > 65_600 ||
                        !value.isNull("record") && value.optJSONObject("record") == null) throw InvalidAccountRecord()
                    if (value.isNull("record")) missing()
                    else available().put("record", value.getJSONObject("record"))
                }
            }
            "checkoutSave" -> {
                val slot = checkoutSlot(command)
                require(command.has("record"))
                val record = command.optJSONObject("record")
                require(command.isNull("record") || record != null)
                require(record == null || record.toString().toByteArray(Charsets.UTF_8).size <= 65_536)
                write(slot, JSONObject().put("record", record ?: JSONObject.NULL), scoped = false)
                available()
            }
            else -> throw IllegalArgumentException("Unknown storage action")
        }
    } catch (_: InvalidAccountRecord) {
        JSONObject().put("status", "invalid")
    } catch (_: Exception) {
        // Missing, inaccessible and corrupt records must never be conflated.
        JSONObject().put("status", "unavailable")
    }

    private fun load(slot: String, field: String, fence: String,
        decode: (JSONObject) -> JSONObject?): JSONObject {
        if (prefs.getBoolean(key(fence), false)) return missing()
        val value = read(slot) ?: return missing()
        val decoded = try { decode(value) } catch (_: Exception) { throw InvalidAccountRecord() } ?: return missing()
        return available().put(field, decoded)
    }

    private fun save(slot: String, value: JSONObject, fence: String) {
        val encoded = encode(value, scoped = true, protected = slot == "session")
        commit(prefs.edit().putString(key(slot), encoded).putBoolean(key(fence), false))
    }

    private fun fence(slot: String, fence: String) {
        // A confirmed durable fence, independent of Keystore access, makes a
        // failed best-effort deletion harmless on the next process launch.
        commit(prefs.edit().putBoolean(key(fence), true))
        runCatching { prefs.edit().remove(key(slot)).commit() }
    }

    private fun restart() {
        val next = UUID.randomUUID().toString()
        cipher("${keyAlias}_$next").encrypt("{}")
        commit(prefs.edit().putString(NAMESPACE, next))
    }

    private fun namespace(): String = prefs.getString(NAMESPACE, "").orEmpty().also {
        require(it.isEmpty() || it.matches(Regex("[0-9a-f-]{36}")))
    }

    private fun key(slot: String): String = namespace().let {
        if (it.isEmpty()) slot else "account_session_${it}_$slot"
    }

    private fun activeCipher(scoped: Boolean): AccountRecordCipher {
        val suffix = if (scoped) namespace() else ""
        return cipher(if (suffix.isEmpty()) keyAlias else "${keyAlias}_$suffix")
    }

    private fun read(slot: String, scoped: Boolean = true): JSONObject? {
        val encoded = prefs.getString(if (scoped) key(slot) else slot, null) ?: return null
        val clear = if (slot == "session") {
            if (!encoded.startsWith("v2.")) return null
            activeCipher(scoped).decrypt(encoded)
        } else {
            if (encoded.startsWith("{")) encoded
            else if (slot.startsWith("account-purchase-v1-")) activeCipher(scoped).decryptPreviousUserRecord(encoded)
            else return null
        }
        return try {
            require(clear.toByteArray(Charsets.UTF_8).size <= 524_288)
            JSONObject(clear)
        } catch (_: Exception) { throw InvalidAccountRecord() }
    }

    private fun encode(value: JSONObject, scoped: Boolean, protected: Boolean): String {
        val clear = value.toString()
        require(clear.toByteArray(Charsets.UTF_8).size <= 524_288)
        return if (protected) activeCipher(scoped).encrypt(clear) else clear
    }

    private fun write(slot: String, value: JSONObject, scoped: Boolean) {
        val encoded = encode(value, scoped, protected = slot == "session")
        commit(prefs.edit().putString(if (scoped) key(slot) else slot, encoded))
    }

    private fun checkoutSlot(command: JSONObject): String = command.getString("slot").also {
        require(it.matches(Regex("account-purchase-v1-[0-9a-f]{64}")))
    }

    private fun commit(editor: SharedPreferences.Editor) {
        if (!editor.commit()) throw SecureStorageUnavailableException()
    }

    private fun available() = JSONObject().put("status", "available")
    private fun missing() = JSONObject().put("status", "missing")
    private class InvalidAccountRecord : Exception()

    private companion object {
        const val NAMESPACE = "account_session_namespace"
    }
}
