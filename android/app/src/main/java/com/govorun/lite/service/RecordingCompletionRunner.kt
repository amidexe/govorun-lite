package com.govorun.lite.service

/**
 * Runs the optional post-recognition delivery phase and guarantees that every
 * terminal path restores the recording surface. This boundary is deliberately
 * independent of Android views so cancellation and failures are reproducible
 * in a local unit test.
 */
internal suspend fun runRecordingCompletion(
    complete: suspend () -> Unit,
    restoreReady: () -> Unit,
) {
    try {
        complete()
    } finally {
        restoreReady()
    }
}
