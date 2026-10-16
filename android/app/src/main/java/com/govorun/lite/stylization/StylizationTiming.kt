package com.govorun.lite.stylization

/** Android displays Core's content-free report; it does not recalculate its stages. */
internal object StylizationTimingFormat {
    val phases = setOf("local", "access", "start", "wait", "finalize")
    data class View(val fields: Map<String, String>, val details: String, val compact: String?)
    data class Presentation(val title: String, val details: String?, val compact: String?,
        val category: String, val attention: Boolean, val success: Boolean)

    fun presentation(value: String?, state: StylizationDiagnosticState?): Presentation {
        val decoded = diagnosticCommand(org.json.JSONObject().put("kind", "presentation")
            .put("value", value ?: org.json.JSONObject.NULL).put("state", state?.wire() ?: org.json.JSONObject.NULL))
        fun optional(key: String) = decoded.optString(key).takeIf { it.isNotEmpty() && it != "null" }
        return Presentation(decoded.getString("title"), optional("details"), optional("shortSummary"),
            decoded.getString("category"), decoded.getBoolean("attention"), decoded.getBoolean("success"))
    }

    fun view(value: String?): View? {
        if (value == null) return null
        val decoded = diagnosticCommand(org.json.JSONObject().put("kind", "timing").put("value", value))
        val fields = decoded.optJSONObject("fields")?.takeIf { it.has("phase") } ?: return null
        return View(fields.keys().asSequence().associateWith { fields.getString(it) },
            decoded.getString("details"),
            decoded.optString("shortSummary").takeIf { it.isNotEmpty() && it != "null" })
    }
    fun parse(value: String?): Map<String, String>? = view(value)?.fields
    fun detail(value: String): String = view(value)?.details ?: "Замеры недоступны."
}
