package com.govorun.lite.network

import java.io.Closeable

/**
 * Process-local circuit breaker for every session-bound request to the
 * official Govorun contour. Self-hosted processing is deliberately outside
 * this gate. Sign-out revokes the generation and disconnects official
 * transports before signed-out state is shown.
 */
internal object OfficialSessionNetworkGate {
    private val lock = Any()
    private var generation = 0L
    private var nextID = 0L
    private val active = linkedMapOf<Long, () -> Unit>()

    fun currentEpoch(): Long = synchronized(lock) { generation }

    fun begin(requestAllowed: () -> Boolean): Lease = synchronized(lock) {
        if (!requestAllowed()) throw OfficialSessionRevokedException()
        Lease(generation, requestAllowed)
    }

    fun revokeAll() {
        val cancellations = synchronized(lock) {
            generation = if (generation == Long.MAX_VALUE) 0L else generation + 1L
            active.values.toList().also { active.clear() }
        }
        cancellations.forEach { runCatching(it) }
    }

    internal class Lease(
        private val expectedGeneration: Long,
        private val requestAllowed: () -> Boolean,
    ) {
        fun attach(cancel: () -> Unit): Closeable {
            val id = synchronized(lock) {
                if (generation != expectedGeneration || !requestAllowed()) {
                    throw OfficialSessionRevokedException()
                }
                nextID = if (nextID == Long.MAX_VALUE) 1L else nextID + 1L
                nextID.also { active[it] = cancel }
            }
            return Closeable { synchronized(lock) { active.remove(id) } }
        }

        fun requireCurrent() {
            synchronized(lock) {
                if (generation != expectedGeneration || !requestAllowed()) {
                    throw OfficialSessionRevokedException()
                }
            }
        }
    }
}

internal class OfficialSessionRevokedException : Exception("Official session was revoked")
