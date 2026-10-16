package com.govorun.lite.stylization

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONArray
import org.json.JSONObject

/** Local value conversion only; the core owns Unicode counting and credit admission. */
internal data class StyleTextAdmission(
    val characters: Int,
    val requiredCredits: Int,
    val deadlineMillis: Long,
    val admissionBudgetMillis: Long,
) {
    companion object {
        fun inspect(text: String): StyleTextAdmission {
            val decision = SharedBusiness.evaluate(JSONObject()
                .put("operation", "styleTextAdmission")
                .put("units", JSONArray(text.map { it.code })))
            return StyleTextAdmission(decision.getInt("characters"), decision.getInt("requiredCredits"),
                decision.getLong("deadlineMillis"), decision.getLong("admissionBudgetMillis"))
        }
    }
}
