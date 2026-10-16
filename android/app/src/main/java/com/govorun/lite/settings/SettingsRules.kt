package com.govorun.lite.settings

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal object SettingsRules {
    const val AFTER_PAUSE = "after_pause"
    const val REPEAT_TAP = "repeat_tap"

    fun evaluate(command: JSONObject): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "settingsAction").put("command", command))

    fun recording(action: String, mode: String?, enabled: Boolean = false): JSONObject = evaluate(
        JSONObject().put("kind", "recording").put("action", action)
            .put("mode", mode ?: JSONObject.NULL).put("enabled", enabled))

    fun effects(result: JSONObject): List<JSONObject> = result.getJSONArray("effects").let { items ->
        List(items.length()) { items.getJSONObject(it) }
    }
}
