package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.model.stylization.StylizationFallbackReason
import org.json.JSONObject

internal enum class StylizationDiagnosticOutcome {
    STYLED,
    FALLBACK,
    SKIPPED,
}

internal enum class StylizationDiagnosticReason(val apiId: String) {
    SUCCESS("success"),
    STREAMING_MODE("streaming_mode"),
    CUSTOM_STYLE_DRAFT("custom_style_draft"),
    STYLIZATION_DISABLED("stylization_disabled"),
    TERMS_UPDATE_REQUIRED("terms_update_required"),
    SERVICE_UNAVAILABLE("service_unavailable"),
    ACCESS_INACTIVE("access_inactive"),
    SHORT_PHRASE("short_phrase"),
    TEXT_TOO_LONG("text_too_long"),
    NOT_CONFIGURED("not_configured"),
    APP_RULE_DISABLED("app_rule_disabled"),
    OFFLINE("offline"),
    CONFIRMED_OFFLINE("confirmed_offline"),
    NO_CHANGE("no_change"),
    STYLE_PASS_RESERVE_EMPTY("style_pass_reserve_empty"),
    STYLE_PASS_ISSUE_FAILED("style_pass_issue_failed"),
    STYLE_PASS_RECOVERY_PENDING("style_pass_recovery_pending"),
    ACCOUNT_STATE_CHANGED("account_state_changed"),
    TRANSPORT_FAILED("transport_failed"),
    REQUEST_FAILED("request_failed"),
    SERVER_BUSY("server_busy"),
    SERVER_TIMEOUT("server_timeout"),
    SERVER_UNAVAILABLE("server_unavailable"),
    INVALID_RESPONSE("invalid_response"),
    INSERTION_FAILED("insertion_failed"),
}

internal data class StylizationDiagnosticState(
    val outcome: StylizationDiagnosticOutcome,
    val reason: StylizationDiagnosticReason,
)

internal data class StylizationDiagnosticsSnapshot(
    val last: StylizationDiagnosticState?,
    val pendingFailures: Map<StylizationDiagnosticReason, Int>,
) {
    val pendingFailureCount: Int get() = pendingFailures.values.sum()
}

/**
 * Content-free reliability state.
 *
 * Only the last decision is kept locally. Real online-path failures are held
 * as process-local bounded counters shared by standalone reporting and later
 * anonymous style requests. Pending reports are never restored from disk. There is deliberately no event list, exact time, text, text length,
 * account, device, destination application, pass, token or network address.
 */
internal object StylizationDiagnosticsStore {
    private const val PREFERENCES = "govorun_stylization_diagnostics_v2"
    private const val STATE = "state"
    @Synchronized fun record(context: Context, outcome: StylizationDiagnosticOutcome, reason: StylizationDiagnosticReason) = diagnosticOrElse(Unit) {
        writeState(context, StylizationDiagnosticsSnapshot(StylizationDiagnosticState(outcome, reason), emptyMap()))
        LocalDiagnosticJournal.record(context, StylizationDiagnosticState(outcome, reason))
    }
    @Synchronized fun read(context: Context) = diagnosticOrElse(StylizationDiagnosticsSnapshot(null, emptyMap())) {
        decode(preferences(context).getString(STATE, null))
    }
    @Synchronized fun clearLocalHistory(context: Context) = diagnosticOrElse(Unit) {
        LocalDiagnosticJournal.clear(context)
        preferences(context).edit().remove(STATE).apply()
    }
    @Synchronized fun clear(context: Context) = clearLocalHistory(context)
    internal fun encode(snapshot: StylizationDiagnosticsSnapshot): String = diagnosticCommand(JSONObject()
        .put("kind", "encodeState").put("last", snapshot.last?.wire())).getJSONObject("value").toString()
    internal fun decode(value: String?): StylizationDiagnosticsSnapshot {
        val root = runCatching { JSONObject(value ?: "{}") }.getOrDefault(JSONObject())
        val decoded = diagnosticCommand(JSONObject().put("kind", "decodeState").put("value", root))
        return StylizationDiagnosticsSnapshot(decoded.optJSONObject("last")?.let(::diagnosticState), emptyMap())
    }
    private fun writeState(context: Context, snapshot: StylizationDiagnosticsSnapshot) {
        preferences(context).edit().putString(STATE, encode(snapshot)).apply()
    }
    private fun preferences(context: Context) = context.applicationContext.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
}

internal fun diagnosticReason(fallbackReason: StylizationFallbackReason): StylizationDiagnosticReason =
    StylizationDiagnosticReason.valueOf(diagnosticCommand(JSONObject().put("kind", "fallback").put("reason", fallbackReason.name)).getString("reason"))
