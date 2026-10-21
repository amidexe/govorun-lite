package com.govorun.lite.transcriber

import java.util.concurrent.atomic.AtomicLong

/**
 * Decides who may use the process-wide recognition engine.
 *
 * Govorun's own dictation and the diagnostic check hold it exclusively. External applications
 * share it: several of their sessions may be open at once, and [PhraseQueue] gives them the
 * engine one phrase at a time.
 */
internal class RecognitionSessionArbiter {
    enum class Owner {
        PRIMARY_APP,
        EXTERNAL_APP,
        DIAGNOSTIC,
    }

    data class Lease internal constructor(
        internal val id: Long,
        val owner: Owner,
    )

    private data class ActiveSession(
        val lease: Lease,
        val uid: Int,
        val onPreempt: () -> Unit,
        // Set by a session that rides out Govorun's dictation: told on every change instead of preempted.
        val onPrimaryChange: (() -> Unit)? = null,
    )

    private val lock = Any()
    private val ids = AtomicLong()
    private var exclusive: ActiveSession? = null
    // The primary lease whose microphone is still capturing; recognition of what it heard can outlast it.
    private var capturing: Long? = null
    private val external = mutableListOf<ActiveSession>()

    fun acquireExternal(uid: Int, onPrimaryChange: (() -> Unit)? = null, onPreempt: () -> Unit): Lease? =
        synchronized(lock) {
            if (exclusive != null) return@synchronized null
            if (external.size >= MAX_EXTERNAL_SESSIONS) return@synchronized null
            if (external.count { it.uid == uid } >= MAX_SESSIONS_PER_APP) return@synchronized null
            create(Owner.EXTERNAL_APP, uid, onPreempt, onPrimaryChange).also { external += it }.lease
        }

    fun acquireDiagnostic(): Lease? = synchronized(lock) {
        if (exclusive != null || external.isNotEmpty()) return@synchronized null
        create(Owner.DIAGNOSTIC, 0, {}).also { exclusive = it }.lease
    }

    fun acquirePrimary(): Lease? {
        val displaced: List<() -> Unit>
        val riding: List<() -> Unit>
        val lease =
            synchronized(lock) {
                if (exclusive != null) return null
                val (staying, leaving) = external.partition { it.onPrimaryChange != null }
                displaced = leaving.map { it.onPreempt }
                riding = staying.mapNotNull { it.onPrimaryChange }
                external.removeAll(leaving)
                create(Owner.PRIMARY_APP, 0, {}).also { exclusive = it; capturing = it.lease.id }.lease
            }
        // Never invoke application callbacks while holding the arbiter lock.
        displaced.forEach { runCatching(it) }
        riding.forEach { runCatching(it) }
        return lease
    }

    /** Govorun's microphone stopped (stop, cancel, hidden bubble); its lease may still be recognizing. */
    fun endPrimaryCapture(lease: Lease?) {
        if (lease == null) return
        val riding =
            synchronized(lock) {
                if (capturing != lease.id) return
                capturing = null
                external.mapNotNull { it.onPrimaryChange }
            }
        riding.forEach { runCatching(it) }
    }

    fun release(lease: Lease?) {
        if (lease == null) return
        val riding =
            synchronized(lock) {
                val wasPrimary = exclusive?.lease?.id == lease.id && lease.owner == Owner.PRIMARY_APP
                if (exclusive?.lease?.id == lease.id) exclusive = null
                if (capturing == lease.id) capturing = null
                external.removeAll { it.lease.id == lease.id }
                if (wasPrimary) external.mapNotNull { it.onPrimaryChange } else emptyList()
            }
        riding.forEach { runCatching(it) }
    }

    /** True while Govorun's own dictation is capturing from the microphone. */
    fun primaryCapturing(): Boolean = synchronized(lock) { capturing != null }

    /** True while Govorun's own dictation holds the engine. */
    fun primaryActive(): Boolean = synchronized(lock) { exclusive?.lease?.owner == Owner.PRIMARY_APP }

    fun activeOwner(): Owner? = synchronized(lock) {
        exclusive?.lease?.owner ?: Owner.EXTERNAL_APP.takeIf { external.isNotEmpty() }
    }

    private fun create(owner: Owner, uid: Int, onPreempt: () -> Unit, onPrimaryChange: (() -> Unit)? = null): ActiveSession =
        ActiveSession(Lease(ids.incrementAndGet(), owner), uid, onPreempt, onPrimaryChange)

    companion object {
        // One application may, for example, stream a live recording while a finished one is
        // transcribed. More sessions do not make anything faster: the engine decodes one phrase
        // at a time, and every open session keeps its own buffers.
        const val MAX_SESSIONS_PER_APP = 2
        const val MAX_EXTERNAL_SESSIONS = 4
    }
}

internal object RecognitionSessionCoordinator {
    val process = RecognitionSessionArbiter()
    val phrases = PhraseQueue()
}
