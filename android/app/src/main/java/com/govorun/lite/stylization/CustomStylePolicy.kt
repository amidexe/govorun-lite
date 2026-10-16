package com.govorun.lite.stylization

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

/** View/input adapter to the common Unicode and draft-validation contract. */
internal object CustomStylePolicy {
    val limits: JSONObject get() = SharedBusiness.evaluate(JSONObject().put("operation", "styleLibrary")
        .put("command", JSONObject().put("kind", "limits")))
    val MAX_INSTRUCTION_CHARS: Int get() = limits.getInt("maxInstructionCharacters")
    fun inspect(value: String, field: String = "instruction"): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "styleLibrary").put("command", JSONObject().put("kind", "inspectText")
            .put("field", field).put("value", value)))
    fun normalize(value: String): String = inspect(value).getString("normalized")
    fun isValid(value: String): Boolean = inspect(value).getBoolean("valid")
    fun input(value: String, field: String = "instruction"): String = inspect(value, field).getString("input")
    fun characters(value: String): Int = inspect(value).getInt("characters")
}
