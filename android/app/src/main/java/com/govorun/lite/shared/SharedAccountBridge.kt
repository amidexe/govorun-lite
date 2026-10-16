package com.govorun.lite.shared

import com.govorun.lite.stylization.DiagnosticAttempt
import com.govorun.lite.stylization.diagnosticTiming
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.json.JSONObject

/** Application-scoped I/O adapter for the same account command owner as desktop.
 * A disappearing UI waiter does not destroy an account transaction. Explicit
 * cancel/logout commands revoke it in the shared owner, including network I/O.
 */
internal class SharedAccountBridge(
    terms: Int,
    origin: String,
    private val effectHandler: suspend (JSONObject) -> JSONObject,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val next = AtomicLong(1)
    private val calls = ConcurrentHashMap<Long, CancellableContinuation<JSONObject>>()
    private val diagnostics = ConcurrentHashMap<Long, DiagnosticAttempt>()
    private val effects = ConcurrentHashMap<Long, Job>()
    private val lifecycle = Any()
    private var closing = false
    private var handle: Long

    private val callback = object : SharedAccountCallback {
        override fun effect(id: Long, command: String) {
            val job = scope.launch(start = CoroutineStart.LAZY) {
                try {
                    val value = effectHandler(JSONObject(command))
                    SharedAccountNative.respond(handle, id, value.toString())
                } catch (cancelled: CancellationException) {
                    SharedAccountNative.respond(handle, id,
                        JSONObject().put("error", "account_session_changed").toString())
                    throw cancelled
                } catch (error: Exception) {
                    val code = (error as? SharedBusinessException)?.code ?: "account_invalid_response"
                    SharedAccountNative.respond(handle, id, JSONObject().put("error", code).toString())
                } finally {
                    effects.remove(id)
                }
            }
            effects[id] = job
            job.start()
        }

        override fun cancelEffect(id: Long) { effects.remove(id)?.cancel() }

        override fun complete(id: Long, response: String) {
            val attempt = diagnostics.remove(id)
            val continuation = calls.remove(id) ?: return
            try {
                val value = JSONObject(response)
                value.optString("clientTiming").takeIf { diagnosticTiming(it) != null }
                    ?.let { attempt?.recordTiming(it) }
                if (value.has("error")) {
                    continuation.resumeWithException(SharedBusinessException(value.getString("error"), value.optInt("status"),
                        value.optJSONObject("clientCheckoutFailure"), value.optJSONObject("clientCheckoutRecoveryFailure"),
                        value.optJSONObject("stylizationFailure")))
                } else {
                    continuation.resume(value.getJSONObject("result"))
                }
            } catch (_: Exception) {
                if (continuation.isActive) continuation.resumeWithException(SharedBusinessException("account_invalid_response", 0))
            }
        }
    }

    init {
        SharedBusiness.requireAvailable()
        handle = SharedAccountNative.create(terms, origin, callback)
        check(handle > 0) { "business_bridge_unavailable" }
    }

    suspend fun execute(command: JSONObject): JSONObject = awaitCall(onCancel = { id ->
        if (command.optString("operation").startsWith("stylization") && command.has("sessionGeneration")) {
            synchronized(lifecycle) {
                if (!closing) SharedAccountNative.start(handle, next.getAndIncrement(), JSONObject()
                    .put("operation", "stylizationCancel").put("requestId", id.toString())
                    .put("sessionGeneration", command.getLong("sessionGeneration")).toString())
            }
        }
    }) { id ->
        synchronized(lifecycle) {
            check(!closing) { "account_session_changed" }
            if (command.optString("operation") == "stylizationProcess") {
                DiagnosticAttempt.current?.let { diagnostics[id] = it }
            }
            SharedAccountNative.start(handle, id, command.toString())
        }
    }

    /** Close only when the application owner is disposed, never on screen exit.
     * Rust rejects new work, stops HTTP and finishes protected writes before ack.
     */
    suspend fun close() = withContext(NonCancellable) {
        awaitCall { id ->
            synchronized(lifecycle) {
                check(!closing) { "account_session_changed" }
                closing = true
                SharedAccountNative.close(handle, id)
            }
        }
        scope.cancel()
    }

    private suspend fun awaitCall(onCancel: (Long) -> Unit = {}, start: (Long) -> Unit): JSONObject = suspendCancellableCoroutine { continuation ->
        val id = next.getAndIncrement()
        calls[id] = continuation
        try { start(id) }
        catch (_: LinkageError) {
            diagnostics.remove(id)
            calls.remove(id)?.resumeWithException(SharedBusinessException("business_bridge_unavailable", 0))
        } catch (_: Exception) {
            diagnostics.remove(id)
            calls.remove(id)?.resumeWithException(SharedBusinessException("account_session_changed", 0))
        }
        // Install after start so cancellation cannot overtake JNI registration.
        continuation.invokeOnCancellation {
            diagnostics.remove(id)
            calls.remove(id)
            runCatching { onCancel(id) }
        }
    }
}

internal interface SharedAccountCallback {
    fun effect(id: Long, command: String)
    fun cancelEffect(id: Long)
    fun complete(id: Long, response: String)
}

internal object SharedAccountNative {
    external fun create(terms: Int, origin: String, callback: SharedAccountCallback): Long
    external fun start(handle: Long, call: Long, command: String)
    external fun respond(handle: Long, effect: Long, response: String)
    external fun close(handle: Long, call: Long)
}
