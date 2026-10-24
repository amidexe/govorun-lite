package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.diagnostics.LifecycleDiagnosticReason
import com.govorun.lite.diagnostics.ProcessExitRecord
import com.govorun.lite.diagnostics.unseenProcessExits
import com.govorun.lite.storage.GovorunPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject

internal data class LocalDiagnosticEntry(
    val timeMillis: Long,
    val state: StylizationDiagnosticState? = null,
    val lifecycle: LifecycleDiagnosticReason? = null,
    val timing: String? = null,
    val styleAccess: StyleAccessDiagnostic? = null,
) {
    init {
        require(diagnosticCommand(JSONObject().put("kind", "entry").put("entry", wire())).getBoolean("valid"))
    }
}

/** Local-only journal, separate from anonymous reliability counters and their wire format. */
internal object LocalDiagnosticJournal {
    private const val PREFERENCES = "govorun_local_diagnostic_journal_v1"
    private const val EVENTS = "events"
    private const val EXIT_CHECKPOINT = "process_exit_checkpoint"
    private val revision = MutableStateFlow(0L)
    val changes = revision.asStateFlow()

    @Synchronized
    fun record(context: Context, event: StyleAccessDiagnostic) = diagnosticOrElse(Unit) {
        val entry = LocalDiagnosticEntry(System.currentTimeMillis(), styleAccess = event)
        preferences(context).edit().putString(EVENTS, encode(append(readStored(context), entry))).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun recordTiming(context: Context, timing: String) = diagnosticOrElse(Unit) {
        if (diagnosticTiming(timing) == null) return@diagnosticOrElse
        if (DiagnosticAttempt.current?.recordTiming(timing) == true) return@diagnosticOrElse
        val events = append(readStored(context), LocalDiagnosticEntry(System.currentTimeMillis(), timing = timing))
        preferences(context).edit().putString(EVENTS, encode(events)).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun record(context: Context, state: StylizationDiagnosticState) = diagnosticOrElse(Unit) {
        if (DiagnosticAttempt.current?.record(state) == true) return@diagnosticOrElse
        val events = append(readStored(context), LocalDiagnosticEntry(System.currentTimeMillis(), state))
        preferences(context).edit().putString(EVENTS, encode(events)).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun recordAttempt(context: Context, entry: LocalDiagnosticEntry) = diagnosticOrElse(Unit) {
        preferences(context).edit().putString(EVENTS, encode(append(readStored(context), entry))).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun record(context: Context, reason: LifecycleDiagnosticReason) = diagnosticOrElse(Unit) {
        val events = append(readStored(context), LocalDiagnosticEntry(System.currentTimeMillis(), lifecycle = reason))
        preferences(context).edit().putString(EVENTS, encode(events)).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun importProcessExits(context: Context, records: List<ProcessExitRecord>) = diagnosticOrElse(Unit) {
        val prefs = preferences(context)
        val exits = unseenProcessExits(records, prefs.getLong(EXIT_CHECKPOINT, 0))
        if (exits.isEmpty()) return@diagnosticOrElse
        val entries = readStored(context) + exits.map { LocalDiagnosticEntry(it.timeMillis, lifecycle = it.reason) }
        // Persist the journal and deduplication watermark together, including across journal clears.
        prefs.edit().putString(EVENTS, encode(entries.sortedBy { it.timeMillis }))
            .putLong(EXIT_CHECKPOINT, exits.last().timeMillis).apply()
        revision.value++
        Unit
    }

    @Synchronized
    fun read(context: Context): List<LocalDiagnosticEntry> =
        diagnosticOrElse(emptyList()) { readStored(context) }

    private fun readStored(context: Context) = decode(preferences(context).getString(EVENTS, null))

    @Synchronized
    fun clear(context: Context) = diagnosticOrElse(Unit) {
        val prefs = preferences(context)
        prefs.edit().remove(EVENTS)
            .putLong(EXIT_CHECKPOINT, maxOf(System.currentTimeMillis(), prefs.getLong(EXIT_CHECKPOINT, 0)))
            .apply()
        revision.value++
        Unit
    }

    internal fun append(entries: List<LocalDiagnosticEntry>, next: LocalDiagnosticEntry): List<LocalDiagnosticEntry> =
        journalEntries(diagnosticCommand(JSONObject().put("kind", "append").put("entries", journalArray(entries))
            .put("entry", next.wire())).getJSONArray("entries"))
    internal fun encode(entries: List<LocalDiagnosticEntry>): String = diagnosticCommand(JSONObject()
        .put("kind", "journal").put("entries", journalArray(entries))).getJSONArray("entries").toString()
    internal fun decode(value: String?): List<LocalDiagnosticEntry> {
        val array = runCatching { JSONArray(value ?: "[]") }.getOrDefault(JSONArray())
        return journalEntries(diagnosticCommand(JSONObject().put("kind", "journal").put("entries", array)).getJSONArray("entries"))
    }

    private fun preferences(context: Context) = GovorunPreferences.of(context.applicationContext, PREFERENCES)
}
