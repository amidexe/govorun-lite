package com.govorun.lite.account

/** Serializes logical-session publication independently of token rotation. */
internal class AccountSessionTransitionGate(initialGeneration: Long = 0) {
    private val lock = Any()
    private var generation = initialGeneration

    fun currentGeneration(): Long = synchronized(lock) { generation }

    fun <T> locked(block: (Long) -> T): T = synchronized(lock) { block(generation) }

    fun <T> ifCurrent(expectedGeneration: Long, block: () -> T): T? = synchronized(lock) {
        if (generation != expectedGeneration) return@synchronized null
        block()
    }

    fun <T> advanceIfCurrent(
        expectedGeneration: Long,
        currentSession: () -> Boolean = { true },
        block: (Long) -> T,
    ): T? = synchronized(lock) {
        if (generation != expectedGeneration || !currentSession()) return@synchronized null
        generation += 1
        block(generation)
    }

    fun <T> advance(block: (Long) -> T): T = synchronized(lock) {
        generation += 1
        block(generation)
    }
}
