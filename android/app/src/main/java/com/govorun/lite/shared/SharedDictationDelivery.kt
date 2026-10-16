package com.govorun.lite.shared

import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.CancellationException

/** Decodes local common instructions; it never sends transcript text to a server. */
internal object SharedDictationDelivery {
    val signature: String get() = evaluate(JSONObject().put("kind", "constants")).getString("signature")

    fun prepare(text: String, original: String, marker: String, fallback: Boolean,
        hasOutcomePass: Boolean, latencyKnown: Boolean): DeliveryPlan = try {
        val frame = evaluate(JSONObject().put("kind", "prepare").put("text", units(text))
            .put("original", units(original)).put("marker", marker)
            .put("fallback", fallback).put("hasOutcomePass", hasOutcomePass).put("latencyKnown", latencyKnown))
        DeliveryPlan(frame.getString("text"), frame.getBoolean("insert"),
            effects(frame.getJSONObject("success")), effects(frame.getJSONObject("failure")))
    } catch (cancelled: CancellationException) { throw cancelled
    } catch (_: Exception) {
        // A failed local bridge must not destroy the completed transcript.
        DeliveryPlan(text, text.isNotBlank(), DeliveryEffects(false, true, null), DeliveryEffects(true, true, null))
    }

    fun evaluate(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
        .put("operation", "dictationPreflight").put("command", JSONObject()
            .put("kind", "delivery").put("command", command)))

    fun units(text: String) = JSONArray(text.map { it.code })

    private fun effects(value: JSONObject) = DeliveryEffects(value.getBoolean("recordFailure"),
        value.getBoolean("showFallback"), if (value.isNull("outcome")) null else value.getString("outcome"))
}

internal data class DeliveryPlan(val text: String, val insert: Boolean,
    val success: DeliveryEffects, val failure: DeliveryEffects)
internal data class DeliveryEffects(val recordFailure: Boolean, val showFallback: Boolean, val outcome: String?)
