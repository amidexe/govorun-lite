package com.govorun.lite.shared

import java.time.Instant
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import org.json.JSONObject
internal object SharedLocalModels {
    fun scalar(kind: String, value: String?, legacyEnabled: Boolean = false): String = call(JSONObject()
        .put("kind", kind).put("value", value).put("legacyEnabled", legacyEnabled)).getString("value")
    fun retry(value: String?, policy: String, now: Instant): Long? {
        // JVM parses HTTP date syntax; numeric interpretation and bounds are Core rules.
        val date = runCatching { ZonedDateTime.parse(value?.trim(), DateTimeFormatter.RFC_1123_DATE_TIME).toInstant().toEpochMilli() }.getOrNull()
        val view = call(JSONObject().put("kind", "retry").put("value", value).put("policy", policy)
            .put("dateMillis", date).put("nowMillis", now.toEpochMilli()))
        return if (view.isNull("millis")) null else view.getLong("millis")
    }
    private fun call(c: JSONObject) = SharedBusiness.evaluate(JSONObject().put("operation", "localModelCodec").put("command", c))
}
