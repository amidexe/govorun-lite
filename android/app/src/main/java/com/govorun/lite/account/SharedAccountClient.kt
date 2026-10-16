package com.govorun.lite.account

import com.govorun.lite.shared.SharedAccountBridge
import com.govorun.lite.shared.SharedBusinessException
import java.util.UUID
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.withTimeoutOrNull
import org.json.JSONObject

internal interface AccountSessionReader {
    fun session(): GovorunSession?
    val storageProblem: SecureStorageProblem?
    fun readableSession(): GovorunSession? {
        if (storageProblem != null) throw SecureStorageUnavailableException()
        return session()
    }
}

internal fun interface AccountRequestPort {
    suspend fun send(path: String, method: String, body: JSONObject?, token: String?,
        headers: Map<String, String>, publicRequest: Boolean): AccountHttpResponse
}

/** Application-scoped projection and command adapter. Credentials and all session
 * transitions remain inside AccountSession; native consumers receive an opaque
 * reference that can only be used with this owner and its captured generation.
 */
internal class SharedAccountClient(
    private val scope: CoroutineScope,
    private val origin: String,
    private val terms: Int,
    private val storage: SharedAccountStorage,
    private val onProjection: (JSONObject, GovorunSession?) -> Unit,
    private val onUnavailable: () -> Unit,
    private val transport: AccountHttpPort,
    private val styles: (suspend (JSONObject) -> JSONObject)? = null,
) : AccountSessionReader, AccountRequestPort {
    private val owner = UUID.randomUUID().toString()
    private val intent = AtomicLong()
    private val ready = CompletableDeferred<Unit>()
    @Volatile private var bridge: SharedAccountBridge? = null
    private var started = false
    private val changed = Channel<Unit>(Channel.CONFLATED)
    private data class View(val projection: JSONObject, val session: GovorunSession?)
    @Volatile private var view = View(JSONObject(), null)
    val configured: Boolean get() = origin.isNotBlank() && transport.configured
    val generation: Long get() = view.projection.optJSONObject("snapshot")?.optLong("sessionGeneration") ?: 0
    val projection: JSONObject get() = JSONObject(view.projection.toString())
    override val storageProblem: SecureStorageProblem?
        get() = if (view.projection.optJSONObject("snapshot")?.optBoolean("storageAvailable") == true)
            null else SecureStorageProblem.TEMPORARILY_UNAVAILABLE
    override fun session(): GovorunSession? = view.session

    @Synchronized fun start() {
        if (started) return
        started = true
        scope.launch {
            try {
                check(configured)
                val effects = SharedAccountEffects(storage, transport, ::publish) { command ->
                    checkNotNull(styles).invoke(command)
                }
                val owner = SharedAccountBridge(terms, origin, effects::execute)
                bridge = owner
                // Failed reads are projected by the common owner and may be retried.
                try { owner.execute(JSONObject().put("operation", "restore")) }
                catch (error: SharedBusinessException) {
                    if (error.code !in setOf("account_storage_unavailable", "account_storage_invalid")) throw error
                }
                if (styles != null) {
                    try { owner.execute(JSONObject().put("operation", "stylizationStart")) }
                    catch (error: SharedBusinessException) {
                        if (error.code !in setOf("account_storage_unavailable", "account_storage_invalid")) throw error
                    }
                }
                ready.complete(Unit)
            } catch (error: Exception) {
                ready.completeExceptionally(error)
                if (error is kotlinx.coroutines.CancellationException) throw error
                onUnavailable()
            } catch (error: LinkageError) {
                onUnavailable()
                ready.completeExceptionally(error)
            }
        }
    }

    suspend fun command(value: JSONObject): JSONObject {
        check(configured) { "Account unavailable" }
        ready.await()
        try {
            return checkNotNull(bridge).execute(value)
        } catch (error: SharedBusinessException) {
            // A financial recovery directive carries more information than its
            // underlying storage/transport code. Native DTO adapters preserve it.
            if (error.checkoutFailure != null || error.checkoutRecoveryFailure != null || error.stylizationFailure != null) throw error
            when (error.code) {
                "account_session_changed" -> throw AccountSessionChangedException()
                "account_storage_unavailable", "account_storage_invalid" -> throw SecureStorageUnavailableException()
                else -> throw error
            }
        }
    }

    fun reserveAuthIntent(): Long = intent.incrementAndGet()
    fun isCurrentIntent(expected: Long): Boolean = intent.get() == expected

    suspend fun begin(provider: AuthProvider, requested: Long): String = command(JSONObject().put("operation", "authBegin")
        .put("authIntent", requested).put("provider", provider.apiName)).getString("url")

    suspend fun callback(url: String, installation: String, deviceName: String): JSONObject =
        command(authDevice("authCallback", installation, deviceName).put("url", url))

    suspend fun replaceDevice(id: String, installation: String, deviceName: String): JSONObject =
        command(authDevice("authReplace", installation, deviceName).put("deviceId", id).put("confirmed", true))

    suspend fun cancel() = command(JSONObject().put("operation", "cancelSignIn")
        .put("authIntent", intent.incrementAndGet()))

    suspend fun cancelIfCurrent(expected: Long): Boolean {
        if (!intent.compareAndSet(expected, expected + 1)) return false
        runCatching { command(JSONObject().put("operation", "cancelSignIn").put("authIntent", expected + 1)) }
        return isCurrentIntent(expected + 1)
    }

    suspend fun logout(requested: Long) = command(JSONObject().put("operation", "logout")
        .put("authIntent", requested))

    suspend fun restore() = command(JSONObject().put("operation", "restore"))

    suspend fun close() {
        ready.await()
        bridge?.close()
    }

    suspend fun awaitActivity() {
        while (true) {
            val milliseconds = command(JSONObject().put("operation", "activityDelay")).getLong("milliseconds")
            if (withTimeoutOrNull(milliseconds.coerceAtLeast(1)) { changed.receive() } == null) return
        }
    }

    override suspend fun send(path: String, method: String, body: JSONObject?, token: String?,
        headers: Map<String, String>, publicRequest: Boolean): AccountHttpResponse {
        ready.await()
        val captured = view
        val epoch = captured.projection.getJSONObject("snapshot").getLong("sessionGeneration")
        require(!publicRequest)
        if (token == null || token != captured.session?.accessToken) {
            throw AccountSessionChangedException()
        }
        require(headers.keys.all { it in setOf("If-Match", "If-None-Match", "Idempotency-Key") })
        val response = command(JSONObject().put("operation", "http").put("method", method)
            .put("path", path).put("body", body ?: JSONObject.NULL).put("sessionGeneration", epoch)
            .put("etag", headers["If-Match"] ?: headers["If-None-Match"])
            .put("idempotencyKey", headers["Idempotency-Key"]))
        return AccountHttpResponse(response.getInt("status"), response.opt("body")?.toString().orEmpty(),
            response.optString("etag").takeUnless { response.isNull("etag") },
            (response.opt("retryAfterSeconds") as? Number)?.toLong()?.times(1_000))
    }

    private fun authDevice(operation: String, installation: String, deviceName: String) = JSONObject()
        .put("operation", operation).put("authIntent", intent.get())
        .put("installationId", installation).put("deviceName", deviceName)

    private fun publish(value: JSONObject) {
        val snapshot = value.getJSONObject("snapshot")
        val previous = view.projection.optJSONObject("snapshot")
        if (previous != null && snapshot.getLong("revision") <= previous.getLong("revision")) return
        val token = "shared-session:$owner:${snapshot.getLong("sessionGeneration")}"
        val tokens = value.optJSONObject("tokens")
        // Billing/token publications are not new server balance observations.
        // Reuse the decoded profile until its common revision/freshness changes.
        val sameProfile = previous != null && view.session != null &&
            previous.getLong("sessionGeneration") == snapshot.getLong("sessionGeneration") &&
            previous.getLong("profileRevision") == snapshot.getLong("profileRevision") &&
            previous.getBoolean("profileFresh") == snapshot.getBoolean("profileFresh")
        val session = if (!snapshot.getBoolean("signedIn") || tokens == null) null
        else if (sameProfile) view.session
        else {
            val profile = value.optJSONObject("profile")
            val presentation = profile?.let { JSONObject(it.toString()).apply {
                if (!has("displayName")) put("displayName", tokens.getString("displayName"))
                if (!has("deviceLimit")) put("deviceLimit", tokens.getInt("deviceLimit"))
                if (!has("onlineTermsVersion")) put("onlineTermsVersion", 0)
                getJSONObject("styleAccess").apply {
                    if (!has("type")) put("type", "unknown")
                    if (!has("state")) put("state", "unknown")
                }
            } }
            (presentation?.toProfileSession(token)
                ?: tokens.toSession()).copy(accessToken = token, refreshToken = token)
        }
        view = View(JSONObject(value.toString()), session)
        onProjection(value, session)
        changed.trySend(Unit)
    }
}
