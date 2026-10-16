package com.govorun.lite.shared

import org.json.JSONArray
import org.json.JSONObject

internal object SharedDictationRules {
    val processingLimits: JSONObject get() = evaluate(JSONObject().put("kind", "constants"))
    val minimumCharacters: Int get() = processingLimits.getInt("minimumCharacters")
    fun evaluate(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
        .put("operation", "dictationPreflight").put("command", command))
    fun units(value: String) = JSONArray(value.map { it.code })
    fun selfHosted(command: JSONObject) = evaluate(JSONObject().put("kind", "selfHosted").put("command", command))
}
