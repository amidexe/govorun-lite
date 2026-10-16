package com.govorun.lite.stylization

import android.util.Log
import kotlinx.coroutines.CancellationException

/** Only optional diagnostics: never wrap accounting, storage custody or dictation in this boundary. */
internal inline fun <T> diagnosticOrElse(
    fallback: T,
    reportFailure: (RuntimeException) -> Unit = { Log.w("GovorunDiagnostics", it.javaClass.simpleName) },
    action: () -> T,
): T = try {
    action()
} catch (cancelled: CancellationException) {
    throw cancelled
} catch (failure: RuntimeException) {
    try {
        reportFailure(failure)
    } catch (cancelled: CancellationException) {
        throw cancelled
    } catch (_: RuntimeException) {
        // Failure reporting is optional too. Never report this through the same journal.
    }
    fallback
}
