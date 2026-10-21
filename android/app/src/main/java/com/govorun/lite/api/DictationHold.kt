package com.govorun.lite.api

import android.os.SystemClock
import com.govorun.lite.transcriber.RecognitionSessionCoordinator
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first

/**
 * Lets one live stream ride out Govorun's own dictation. The client is told when the bird's
 * microphone started and stopped capturing (fact and time only); while it captures, the stream's
 * audio is skipped. The stream's phrases wait for the engine until the bird has recognized its own.
 */
internal class DictationHold(
    private val notify: (active: Boolean, elapsedRealtimeMs: Long) -> Unit,
    private val capturing: () -> Boolean = { RecognitionSessionCoordinator.process.primaryCapturing() },
    private val engineHeld: () -> Boolean = { RecognitionSessionCoordinator.process.primaryActive() },
    private val clock: () -> Long = SystemClock::elapsedRealtime,
) {
    private val leased = MutableStateFlow(false)

    @Volatile var isHeld = false
        private set

    /** Dictations begun so far: a reader that sees it change knows a dictation came between two windows. */
    @Volatile var generation = 0L
        private set

    /** Follows the arbiter, so notifications that arrive out of order still end in the true state. */
    @Synchronized
    fun sync() {
        leased.value = engineHeld()
        val now = capturing()
        if (now == isHeld) return
        isHeld = now
        if (now) generation++
        notify(now, clock())
    }

    suspend fun awaitReleased() {
        leased.first { !it }
    }
}
