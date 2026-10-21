package com.govorun.lite.service

import android.content.Context
import com.govorun.lite.model.stylization.StylizationClientOutcome
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.StylizationResult
import com.govorun.lite.stylization.StylizationDiagnosticOutcome
import com.govorun.lite.stylization.StylizationDiagnosticReason
import com.govorun.lite.stylization.StylizationDiagnosticsStore
import com.govorun.lite.stylization.DiagnosticAttempt
import com.govorun.lite.util.VoiceMarkerPrefs

internal fun createStylizedTextDelivery(
    context: Context,
    process: suspend (String, StylizationInputMode, String?) -> StylizationResult,
    insert: (String) -> Boolean,
    reportClientOutcome: (String, StylizationClientOutcome, String) -> Unit,
    showFallback: () -> Unit,
): StylizedTextDelivery = StylizedTextDelivery(
    process = process,
    insert = insert,
    reportClientOutcome = reportClientOutcome,
    showFallback = showFallback,
    voiceMarkerMode = { VoiceMarkerPrefs.deliveryMarker(context) },
    collectAttempt = { action -> DiagnosticAttempt.collect(context, action) },
    rememberOriginal = { original ->
        LastDictation.attachOriginal(context, com.govorun.lite.transcriber.Dictionary.applyReplacements(context, original))
    },
    recordInsertionFailure = {
        StylizationDiagnosticsStore.record(
            context,
            StylizationDiagnosticOutcome.FALLBACK,
            StylizationDiagnosticReason.INSERTION_FAILED,
        )
    },
)

internal fun BubbleOverlayController.RecordingUiMode.stylizationInputMode(): StylizationInputMode = when (this) {
    BubbleOverlayController.RecordingUiMode.LEGACY -> StylizationInputMode.AUTO
    BubbleOverlayController.RecordingUiMode.CONFIRM -> StylizationInputMode.CONFIRM
    BubbleOverlayController.RecordingUiMode.HOLD -> StylizationInputMode.HOLD
}
