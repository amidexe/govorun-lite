package com.govorun.lite.stylization

import com.govorun.lite.account.GovorunAccountRepository
import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import org.json.JSONObject

/** Carries Core's opaque request ID for one microphone session. Never blocks capture. */
internal class PreparedStylesConnection(
    scope: CoroutineScope,
    private val begin: suspend () -> String? = {
        val result = GovorunAccountRepository.sharedAccountCommand(JSONObject()
            .put("operation", "stylizationBeginRecording")
            .put("sessionGeneration", GovorunAccountRepository.state.value.sessionGeneration))
        result.optString("requestId").takeUnless { result.isNull("requestId") }
    },
    private val finish: suspend (String) -> Unit = { id ->
        GovorunAccountRepository.sharedAccountCommand(JSONObject()
            .put("operation", "stylizationEndRecording").put("requestId", id))
        Unit
    },
) : AbstractCoroutineContextElement(Key) {
    companion object Key : CoroutineContext.Key<PreparedStylesConnection>
    private val ownerScope = scope
    @Volatile private var closed = false
    private val endSent = AtomicBoolean(false)
    private val pending = scope.async(Dispatchers.IO) {
        var id: String? = null
        try {
            id = begin()
            id
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { null }
        finally { if (closed && id != null) withContext(NonCancellable) { end(id) } }
    }
    suspend fun requestId(): String? = if (pending.isCompleted && !pending.isCancelled) pending.await() else null
    suspend fun <T> deliver(block: suspend () -> T): T = withContext(this) { block() }
    fun close() {
        closed = true
        ownerScope.launch(Dispatchers.IO) {
            try { pending.await()?.let { end(it) } }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { /* optional preparation never changes the recording result */ }
        }
    }
    private suspend fun end(id: String) {
        if (!endSent.compareAndSet(false, true)) return
        try { finish(id) }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { /* the transport owns the bounded connection lifetime */ }
    }
}
