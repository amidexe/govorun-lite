package com.govorun.lite.service

import com.govorun.lite.shared.SharedDictationDelivery
import org.json.JSONObject
import java.util.concurrent.CancellationException

/** Holds native text fragments and executes the shared streaming instructions. */
internal class StreamingVoiceMarker(private val marker: String) {
    private var state: JSONObject? = null
    private val pending = mutableListOf<String>()
    private var bridgeFailed = false

    fun accept(fragment: String): String? = step(JSONObject().put("kind", "fragment")
        .put("units", SharedDictationDelivery.units(fragment)), fragment)

    fun flush(hadText: Boolean): String? = step(JSONObject().put("kind", "flush").put("hadText", hadText))

    fun suffix(hadText: Boolean): String? = step(JSONObject().put("kind", "suffix").put("hadText", hadText))

    private fun step(event: JSONObject, emergencyText: String = ""): String? {
        try {
            check(!bridgeFailed)
            val frame = SharedDictationDelivery.evaluate(JSONObject().put("kind", "stream")
                .put("state", state ?: JSONObject.NULL).put("marker", marker).put("event", event))
            state = frame.getJSONObject("state")
            val fragment = frame.getString("fragment")
            return when (frame.getString("action")) {
                "skip" -> null
                "buffer" -> { pending += fragment; null }
                "insert" -> fragment
                "release" -> {
                    if (fragment.isNotEmpty()) pending += fragment
                    frame.getString("prefix") + drain()
                }
                else -> error("invalid_delivery_instruction")
            }
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            // Release buffered local text once, then keep inserting without optional decoration.
            bridgeFailed = true
            if (emergencyText.isNotBlank()) pending += emergencyText
            return drain().takeIf(String::isNotBlank)
        }
    }

    private fun drain() = pending.joinToString(" ").also { pending.clear() }
}
