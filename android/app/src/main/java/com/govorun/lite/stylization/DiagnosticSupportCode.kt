package com.govorun.lite.stylization
import org.json.JSONObject
internal object DiagnosticSupportCode {
    fun result(value: String): String? = code(value, false)
    fun reason(value: String): String? = code(value, true)
    private fun code(value: String, legacy: Boolean): String? {
        val view = diagnosticCommand(JSONObject().put("kind", "support").put("value", value).put("legacy", legacy))
        return if (view.isNull("code")) null else view.getString("code")
    }
}
