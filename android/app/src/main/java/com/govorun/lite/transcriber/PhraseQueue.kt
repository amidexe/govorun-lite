package com.govorun.lite.transcriber

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred

/**
 * Gives external sessions the engine one phrase (at most 30 s of audio) at a time.
 *
 * A phrase someone is waiting to see (a live stream) goes before a phrase of a prepared
 * recording; within each kind phrases go in arrival order. A file therefore never blocks a
 * live stream for longer than one phrase, and two files share the engine phrase by phrase.
 * The rule is about who waits, not about the model, so it holds for any engine that decodes
 * audio in pieces.
 */
internal class PhraseQueue {
    enum class Kind { LIVE, FILE }

    private val lock = Any()
    private var busy = false
    private val live = ArrayDeque<CompletableDeferred<Unit>>()
    private val files = ArrayDeque<CompletableDeferred<Unit>>()

    suspend fun <T> decode(kind: Kind, block: suspend () -> T): T {
        val turn = CompletableDeferred<Unit>()
        synchronized(lock) {
            if (!busy) {
                busy = true
                turn.complete(Unit)
            } else if (kind == Kind.LIVE) {
                live.addLast(turn)
            } else {
                files.addLast(turn)
            }
        }
        try {
            turn.await()
        } catch (cancelled: CancellationException) {
            synchronized(lock) {
                // Not found in either line means the turn was already handed over: pass it on.
                if (!live.remove(turn) && !files.remove(turn)) handOver()
            }
            throw cancelled
        }
        try {
            return block()
        } finally {
            synchronized(lock) { handOver() }
        }
    }

    private fun handOver() {
        val next = live.removeFirstOrNull() ?: files.removeFirstOrNull()
        if (next == null) busy = false else next.complete(Unit)
    }
}
