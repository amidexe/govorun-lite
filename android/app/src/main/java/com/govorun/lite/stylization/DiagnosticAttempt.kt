package com.govorun.lite.stylization

import android.content.Context
import kotlinx.coroutines.asContextElement
import kotlinx.coroutines.withContext

/** Correlates one coroutine's local outcome and timing without persisted identifiers. */
internal class DiagnosticAttempt {
    private var state: StylizationDiagnosticState? = null
    private var timing: String? = null
    private var deliveryTiming: String? = null
    private var sentCharacters: Int? = null
    private var closed = false

    @Synchronized fun record(value: StylizationDiagnosticState): Boolean {
        if (closed) return false
        state = value
        return true
    }

    @Synchronized fun recordTiming(value: String): Boolean {
        if (closed) return false
        timing = value
        return true
    }

    @Synchronized fun recordDeliveryTiming(value: String) { if (!closed) deliveryTiming = value }
    @Synchronized fun recordRequestCharacters(value: Int) { if (!closed) sentCharacters = value }

    @Synchronized fun finish(timeMillis: Long): LocalDiagnosticEntry? {
        if (closed) return null
        closed = true
        if (state == null && timing == null) return null
        val trace = deliveryTiming?.takeUnless { timing?.startsWith("self_hosted ") == true }?.let { full ->
            val online = timing ?: "timing_v1 phase=local result=success deadline_ms=0 elapsed_ms=0 local_ms=0 start_ms=0 wait_ms=0 finalize_ms=0 starts=0 polls=0 transport_errors=0 transport_timeouts=0 http_errors=0"
            "$online $full" + (sentCharacters?.let { " sent_chars=$it" } ?: "")
        } ?: timing
        return LocalDiagnosticEntry(timeMillis, state = state, timing = trace)
    }

    companion object {
        private val scope = ThreadLocal<DiagnosticAttempt?>()
        val current: DiagnosticAttempt? get() = scope.get()

        suspend fun <T> collect(context: Context, action: suspend () -> T): T =
            collect({ LocalDiagnosticJournal.recordAttempt(context, it) }, action)

        internal suspend fun <T> collect(write: (LocalDiagnosticEntry) -> Unit, action: suspend () -> T): T {
            // Delivery owns the outer attempt: write the journal after insertion,
            // not when its nested processing call returns the server result.
            if (current != null) return action()
            val attempt = DiagnosticAttempt()
            return withContext(scope.asContextElement(attempt)) {
                try { action() }
                finally {
                    try {
                        attempt.finish(System.currentTimeMillis())?.let {
                            write(it)
                        }
                    } catch (_: Exception) { /* Diagnostics cannot change delivery. */ }
                }
            }
        }
    }
}
