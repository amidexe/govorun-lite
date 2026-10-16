package com.govorun.lite.service

import com.govorun.lite.shared.SharedDictationRules
import org.json.JSONObject

/** Native lifetime and text buffer; release decisions come from the common module. */
internal class TranscriptDeliverySession(private val deferred: Boolean) {
    var connection: com.govorun.lite.stylization.PreparedStylesConnection? = null
    suspend fun deliver(block: suspend () -> Unit) { connection?.deliver(block) ?: block() }
    private var state: JSONObject? = null
    private var closed = false
    private var bridgeFailed = false
    private val fragments = mutableListOf<String>()
    var fragmentCount = 0
        private set

    fun minimumSilenceSeconds(selectedPause: () -> Float) = selectedPause()

    fun accept(text: String): String? {
        if (closed) return null
        try {
            check(!bridgeFailed)
            val frame = step(JSONObject().put("kind", "fragment").put("units", SharedDictationRules.units(text)))
            val fragment = frame.getString("fragment")
            return when (frame.getString("action")) {
                "ignore" -> null
                "insert" -> fragment
                "buffer" -> { fragments += fragment; null }
                else -> error("invalid_transcript_instruction")
            }
        } catch (_: Exception) {
            // A broken local bridge can delay delivery, never insert unconfirmed text.
            bridgeFailed = true
            if (text.isNotBlank()) { fragments += text; fragmentCount++ }
            return null
        }
    }

    fun complete(): String? {
        if (closed) return null
        closed = true
        return try {
            check(!bridgeFailed)
            val frame = step(JSONObject().put("kind", "complete"))
            if (frame.getString("action") == "release") drain(frame.getString("separator")) else null
        } catch (_: Exception) { drain(" ") }
    }

    fun discard() {
        connection?.close()
        closed = true
        fragments.clear()
        fragmentCount = 0
        state = null
    }

    private fun step(event: JSONObject): JSONObject {
        val frame = SharedDictationRules.evaluate(JSONObject().put("kind", "transcript")
            .put("state", state ?: JSONObject.NULL).put("deferred", deferred).put("event", event))
        state = frame.getJSONObject("state")
        fragmentCount = state!!.getInt("fragmentCount")
        return frame
    }

    private fun drain(separator: String) = fragments.joinToString(separator).takeIf(String::isNotBlank)
        .also { fragments.clear() }
}
