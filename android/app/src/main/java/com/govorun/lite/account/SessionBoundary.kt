package com.govorun.lite.account

import java.util.concurrent.CancellationException

/** Shares the account owner's lock so checking identity and publishing are atomic. */
class SessionBoundary<Identity>(
    private val current: () -> Identity,
    private val serialize: (() -> Unit) -> Unit,
) {
    fun capture(): Lease {
        var result: Lease? = null
        serialize { result = Lease(current()) }
        return checkNotNull(result)
    }

    inner class Lease internal constructor(val identity: Identity) {
        fun isCurrent(): Boolean {
            var matches = false
            serialize { matches = current() == identity }
            return matches
        }

        fun ensureCurrent() {
            if (!isCurrent()) throw CancellationException("Account session changed")
        }

        fun publish(block: () -> Unit): Boolean {
            var accepted = false
            serialize {
                if (current() == identity) {
                    block()
                    accepted = true
                }
            }
            return accepted
        }

        /** A stale failure is cancellation, never an error belonging to the new account. */
        suspend fun <T> request(block: suspend () -> T): T {
            ensureCurrent()
            val result = try {
                block()
            } catch (error: Throwable) {
                ensureCurrent()
                throw error
            }
            ensureCurrent()
            return result
        }

        suspend fun <T> result(block: suspend () -> T): SessionResult<T> {
            val result = try {
                Result.success(request(block))
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                Result.failure(error)
            }
            return SessionResult(result, ::publish)
        }
    }
}

/** UI callbacks may run later than the request; they retain the same session lease. */
class SessionResult<T> internal constructor(
    private val result: Result<T>,
    private val publish: (() -> Unit) -> Boolean,
) {
    fun onSuccess(block: (T) -> Unit): SessionResult<T> = apply {
        result.onSuccess { value -> publish { block(value) } }
    }

    fun onFailure(block: (Throwable) -> Unit): SessionResult<T> = apply {
        result.onFailure { error -> publish { block(error) } }
    }
}
