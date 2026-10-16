package com.govorun.lite.service

import com.govorun.lite.shared.SharedDictationDelivery

import com.govorun.lite.model.stylization.StylizationFallbackReason
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.StylizationResult
import com.govorun.lite.model.stylization.StylizationClientOutcome
import java.util.concurrent.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive

internal class StylizedTextDelivery(
    private val process: suspend (String, StylizationInputMode, String?) -> StylizationResult,
    private val insert: (String) -> Boolean,
    private val reportClientOutcome: (String, StylizationClientOutcome, String) -> Unit,
    private val showFallback: () -> Unit,
    private val voiceMarkerMode: () -> String,
    private val recordInsertionFailure: () -> Unit,
    private val collectAttempt: suspend (suspend () -> Unit) -> Unit = { it() },
) {
    suspend fun deliver(
        text: String,
        inputMode: StylizationInputMode,
        appendVoiceMarker: Boolean = false,
        packageName: String? = null,
    ) = collectAttempt {
        val result = safelyProcess(text, inputMode, packageName)
        val plan = SharedDictationDelivery.prepare(result.text, text,
            if (appendVoiceMarker) voiceMarkerMode() else "off",
            result.usedFallback, result.outcomePass != null, result.latencyBucket != "unknown")
        currentCoroutineContext().ensureActive()
        val inserted = plan.insert && attemptTextInsertion(insert = { insert(plan.text) })
        val effects = if (inserted) plan.success else plan.failure
        if (effects.recordFailure) recordInsertionFailure()
        val outcomePass = result.outcomePass
        if (outcomePass != null && effects.outcome != null) {
            val outcome = StylizationClientOutcome.entries.single { it.apiId == effects.outcome }
            reportClientOutcome(outcomePass, outcome, result.latencyBucket)
        }
        if (effects.showFallback) showFallback()
    }

    /**
     * The local transcript is already complete when this boundary is entered.
     * An unexpected optional-processing failure therefore degrades to that
     * transcript instead of escaping and leaving the overlay in PROCESSING.
     * Lifecycle cancellation is different: the owning service is going away
     * and must not paste into a possibly unrelated editor.
     */
    private suspend fun safelyProcess(
        text: String,
        inputMode: StylizationInputMode,
        packageName: String?,
    ): StylizationResult = try {
        process(text, inputMode, packageName)
    } catch (error: CancellationException) {
        throw error
    } catch (_: Exception) {
        StylizationResult(
            text = text,
            usedFallback = true,
            fallbackReason = StylizationFallbackReason.REQUEST_FAILED,
        )
    }
}
