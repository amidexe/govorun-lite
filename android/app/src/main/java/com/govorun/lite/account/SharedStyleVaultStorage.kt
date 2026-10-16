package com.govorun.lite.account

import android.content.Context
import android.content.SharedPreferences
import java.security.MessageDigest
import org.json.JSONObject

/** Private plain pass-pool I/O. Core supplies the current format and CAS. */
internal class SharedStyleVaultStorage(
    private val prefs: SharedPreferences,
) {
    constructor(context: Context) : this(
        context.applicationContext.getSharedPreferences("govorun_style_pass_pool_v1", Context.MODE_PRIVATE),
    )

    private var cachedStored: String? = null
    private var cachedRaw: String? = null
    private var cachedRevision: String? = null

    @Synchronized fun execute(command: JSONObject): JSONObject = try {
        val stored = prefs.getString(KEY, null)
        // Check the physical slot on every call: another writer or removal must invalidate the decoded record before a CAS decision.
        if (stored != cachedStored) {
            val raw = stored
            remember(stored, raw, raw?.let(::revision))
        }
        val raw = cachedRaw
        val revision = cachedRevision
        when (command.getString("action")) {
            "revision" -> if (raw == null) status("missing") else available(revision!!)
            "read" -> if (raw == null) status("missing") else available(revision!!).put("raw", raw)
            "write" -> {
                val expected = command.optString("expectedRevision").takeUnless { command.isNull("expectedRevision") }
                if (revision != expected) status("conflict") else {
                    val value = command.getString("raw")
                    require(value.toByteArray(Charsets.UTF_8).size <= 256 * 1024)
                    val encoded = value
                    if (prefs.edit().putString(KEY, encoded).commit()) {
                        val next = revision(value)
                        remember(encoded, value, next)
                        available(next)
                    } else {
                        remember(null, null, null)
                        status("unavailable")
                    }
                }
            }
            else -> status("invalid")
        }
    } catch (_: Exception) {
        remember(null, null, null)
        status("unavailable")
    }

    private fun remember(stored: String?, raw: String?, revision: String?) {
        cachedStored = stored
        cachedRaw = raw
        cachedRevision = revision
    }

    private fun revision(raw: String): String = MessageDigest.getInstance("SHA-256")
        .digest(raw.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }
    private fun status(value: String) = JSONObject().put("status", value)
    private fun available(revision: String) = status("available").put("revision", revision)
    private companion object { const val KEY = "pool" }
}
