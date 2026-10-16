package com.govorun.lite.transcriber

import java.util.concurrent.atomic.AtomicLong

/** Serializes access to the process-wide recognition engine. */
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
        val onPreempt: () -> Unit,
    )

    private val lock = Any()
    private val ids = AtomicLong()
    private var active: ActiveSession? = null

    fun acquireExternal(onPreempt: () -> Unit): Lease? =
        synchronized(lock) {
            if (active != null) return@synchronized null
            create(Owner.EXTERNAL_APP, onPreempt).also { active = it }.lease
        }

    fun acquireDiagnostic(): Lease? = synchronized(lock) {
        if (active != null) return@synchronized null
        create(Owner.DIAGNOSTIC, {}).also { active = it }.lease
    }

    fun acquirePrimary(): Lease? {
        var displaced: (() -> Unit)? = null
        val lease =
            synchronized(lock) {
                when (active?.lease?.owner) {
                    Owner.PRIMARY_APP, Owner.DIAGNOSTIC -> return null
                    Owner.EXTERNAL_APP -> displaced = active?.onPreempt
                    null -> Unit
                }
                create(Owner.PRIMARY_APP, {}).also { active = it }.lease
            }
        // Never invoke application callbacks while holding the arbiter lock.
        displaced?.let { runCatching(it) }
        return lease
    }

    fun release(lease: Lease?) {
        if (lease == null) return
        synchronized(lock) {
            if (active?.lease?.id == lease.id) active = null
        }
    }

    fun activeOwner(): Owner? = synchronized(lock) { active?.lease?.owner }

    private fun create(owner: Owner, onPreempt: () -> Unit): ActiveSession =
        ActiveSession(Lease(ids.incrementAndGet(), owner), onPreempt)
}

internal object RecognitionSessionCoordinator {
    val process = RecognitionSessionArbiter()
}
