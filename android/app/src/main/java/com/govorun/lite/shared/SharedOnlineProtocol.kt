package com.govorun.lite.shared

import org.json.JSONObject

internal object SharedOnlineProtocol {
    fun sampleAge(status: Int, body: String, date: String?, modified: String?): Long? {
        val result = SharedDictationRules.evaluate(JSONObject().put("kind", "onlineProtocol")
            .put("command", JSONObject().put("kind", "proof").put("status", status).put("body", body)
                .put("date", date ?: JSONObject.NULL).put("modified", modified ?: JSONObject.NULL)))
        return if (result.getBoolean("healthy")) result.getLong("sampleAge") else null
    }
}
