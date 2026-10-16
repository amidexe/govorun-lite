package com.govorun.lite.stylization

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject
import org.json.JSONArray
internal fun diagnosticCommand(command: JSONObject): JSONObject = SharedBusiness.evaluate(JSONObject()
    .put("operation", "androidDiagnostics").put("command", command))
internal fun diagnosticTiming(value: String?): JSONObject? = diagnosticCommand(JSONObject()
    .put("kind", "timing").put("value", value)).optJSONObject("fields")
internal fun diagnosticState(v: JSONObject): StylizationDiagnosticState = StylizationDiagnosticState(
    StylizationDiagnosticOutcome.valueOf(v.getString("outcome")), StylizationDiagnosticReason.valueOf(v.getString("reason")))
internal fun StylizationDiagnosticState.wire() = JSONObject().put("outcome", outcome.name).put("reason", reason.name)
internal fun diagnosticCounts(v: JSONObject): Map<StylizationDiagnosticReason, Int> =
    v.keys().asSequence().associate { StylizationDiagnosticReason.valueOf(it) to v.getInt(it) }
internal fun Map<StylizationDiagnosticReason, Int>.wire() = JSONObject().apply { forEach { (k, v) -> put(k.name, v) } }
internal fun LocalDiagnosticEntry.wire(): JSONObject = JSONObject().put("time", timeMillis).apply {
    state?.let { put("outcome", it.outcome.name).put("reason", it.reason.name) }; lifecycle?.let { put("lifecycle", it.name) }
    timing?.let { put("timing", it) }; styleAccess?.let { put("styleAccess", it.name) }
}
internal fun journalArray(entries: List<LocalDiagnosticEntry>) = JSONArray(entries.map { it.wire() })
internal fun journalEntries(a: JSONArray): List<LocalDiagnosticEntry> = (0 until a.length()).map { index ->
    val v = a.getJSONObject(index)
    LocalDiagnosticEntry(v.getLong("time"), if (v.has("outcome")) diagnosticState(v) else null,
        if (v.has("lifecycle")) com.govorun.lite.diagnostics.LifecycleDiagnosticReason.valueOf(v.getString("lifecycle")) else null,
        v.optString("timing").takeIf { v.has("timing") },
        if (v.has("styleAccess")) StyleAccessDiagnostic.valueOf(v.getString("styleAccess")) else null)
}
