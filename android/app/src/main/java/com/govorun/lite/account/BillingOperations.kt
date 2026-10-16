package com.govorun.lite.account

import com.govorun.lite.account.SessionBoundary
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal fun billingSessionBoundary(
    transitions: AccountSessionTransitionGate,
    current: () -> AccountState,
) = SessionBoundary(
    current = { billingSessionScope(current()) },
    serialize = { block -> transitions.locked { block() } },
)

/** Coordinates mutations separately from refreshes; each operation retains its original session. */
internal class BillingOperations(
    private val boundary: SessionBoundary<BillingSessionScope>,
    private val read: () -> BillingUiState,
    private val write: (BillingUiState) -> Unit,
) {
    private val mutations = Mutex()

    suspend fun <T> run(mutating: Boolean = true, block: suspend BillingOperation.() -> T): T {
        val lease = boundary.capture()
        suspend fun execute(): T {
            val operation = BillingOperation(lease, read, write)
            return try {
                lease.request { operation.block() }
            } catch (cancelled: CancellationException) {
                operation.cancel()
                throw cancelled
            }
        }
        return if (mutating) mutations.withLock { execute() } else execute()
    }
}

internal class BillingOperation(
    private val lease: SessionBoundary<BillingSessionScope>.Lease,
    private val read: () -> BillingUiState,
    private val write: (BillingUiState) -> Unit,
) {
    private var ownsWorking = false
    val scope get() = lease.identity
    var ui: BillingUiState
        get() { lease.ensureCurrent(); return read() }
        set(value) {
            if (!lease.publish { write(value) }) throw CancellationException("Billing session changed")
        }

    /** Serialize protected I/O with the existing identity transition lock. */
    fun <T> storage(block: () -> T): T {
        var result: Result<T>? = null
        if (!lease.publish { result = runCatching(block) }) throw CancellationException("Billing session changed")
        return checkNotNull(result).getOrThrow()
    }

    fun isCurrent() = lease.isCurrent()
    fun ensureCurrent() = lease.ensureCurrent()
    suspend fun <T> request(block: suspend () -> T): T = lease.request(block)

    fun update(change: (BillingUiState) -> BillingUiState) {
        if (!lease.publish { write(change(read())) }) throw CancellationException("Billing session changed")
    }

    fun finishRefresh() {
        lease.publish { write(read().copy(loading = false)) }
    }

    fun working(value: Boolean, message: String? = null, messageIsPrimary: Boolean = false,
        preserveFormFailure: Boolean = false) {
        ownsWorking = value
        update { it.copy(working = value, message = message, checkoutMessage = message,
            messageIsPrimary = messageIsPrimary,
            checkoutFormFailure = it.checkoutFormFailure.takeIf { preserveFormFailure }) }
    }

    fun cancel() {
        if (ownsWorking) lease.publish { write(read().copy(working = false)) }
    }
}

