package com.govorun.lite.shared

import org.json.JSONArray
import org.json.JSONObject

internal object SharedDictionary {
    fun evaluate(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
        .put("operation", "dictionary").put("command", command))

    fun validate(raw: String) {
        evaluate(JSONObject().put("kind", "parse").put("raw", raw))
    }

    fun apply(text: String, raw: String, enabled: Boolean): String = evaluate(JSONObject()
        .put("kind", "apply").put("raw", raw).put("text", text).put("enabled", enabled)).getString("text")

    fun strings(values: List<String>) = JSONArray(values)
    fun variants(command: JSONObject): List<String> = evaluate(command).getJSONArray("variants").let { values ->
        List(values.length()) { values.getString(it) }
    }
}
