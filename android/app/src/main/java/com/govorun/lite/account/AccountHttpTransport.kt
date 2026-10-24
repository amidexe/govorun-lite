package com.govorun.lite.account

import com.govorun.lite.BuildConfig
import com.govorun.lite.network.OfficialSessionNetworkGate
import java.io.Closeable
import java.io.InputStream
import java.io.IOException
import java.net.URI
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.Call
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody
import okio.BufferedSink
import org.json.JSONObject

/** HTTP I/O only. Session admission, token rotation and retries belong to its caller.
 * Cancellation disconnects the call; redirects, cookies and implicit POST retries
 * remain disabled for both the account protocol and existing API adapters.
 */
internal class AccountHttpTransport(
    private val baseUrl: String,
    private val allowInsecureLoopbackForTests: Boolean = false,
    private val onNetworkResult: (Boolean) -> Unit = {},
    private val networkAllowed: () -> Boolean = { true },
) : AccountHttpPort {
    override val configured: Boolean = runCatching { validatedBaseUrl() }.isSuccess

    override suspend fun send(path: String, method: String, body: JSONObject?,
        accessToken: String?, headers: Map<String, String>): AccountHttpResponse =
        send(path, method, body, accessToken, headers, networkLease = null)

    suspend fun send(
        path: String,
        method: String,
        body: JSONObject? = null,
        accessToken: String? = null,
        headers: Map<String, String> = emptyMap(),
        networkLease: OfficialSessionNetworkGate.Lease?,
    ): AccountHttpResponse {
        require(path.startsWith("/") && !path.startsWith("//"))
        // Offline or roaming choice: fail like a missing network, before any socket.
        if (!networkAllowed()) throw IOException("Network refused by the user's choice")
        return suspendCancellableCoroutine { continuation ->
            val callRef = AtomicReference<Call?>()
            val worker = REQUEST_SCOPE.launch {
                var leaseAttachment: Closeable? = null
                try {
                    networkLease?.requireCurrent()
                    val bodyBytes = body?.toString()?.toByteArray(Charsets.UTF_8)
                        ?: if (method in setOf("POST", "PUT", "PATCH")) ByteArray(0) else null
                    // Also stop HTTP 503 / Retry-After: 0 follow-ups: disabling connection
                    // retries alone does not prevent OkHttp from repeating a request body.
                    val requestBody = bodyBytes?.let { bytes -> object : RequestBody() {
                        override fun contentType() = JSON_MEDIA_TYPE
                        override fun contentLength() = bytes.size.toLong()
                        override fun isOneShot() = true
                        override fun writeTo(sink: BufferedSink) { sink.write(bytes) }
                    } }
                    val request = Request.Builder().url(validatedBaseUrl() + path)
                        .method(method, requestBody)
                        .header("Accept", "application/json")
                        .header("Cache-Control", "no-store")
                        .header("X-Govorun-App-Version", BuildConfig.VERSION_NAME)
                        .apply {
                            if (accessToken != null) header("Authorization", "Bearer $accessToken")
                            headers.forEach { (name, value) -> header(name, value) }
                        }.build()
                    val call = HTTP.newCall(request)
                    callRef.set(call)
                    leaseAttachment = networkLease?.attach {
                        callRef.getAndSet(null)?.cancel()
                    }
                    if (!continuation.isActive) return@launch
                    networkLease?.requireCurrent()
                    val result = call.execute().use { response ->
                        val responseBody = response.body.byteStream().readLimited()
                        networkLease?.requireCurrent()
                        AccountHttpResponse(response.code, responseBody, response.header("ETag"),
                            accountRetryAfterMillis(response.header("Retry-After")))
                    }
                    if (continuation.isActive) {
                        onNetworkResult(true)
                        continuation.resume(result)
                    }
                } catch (error: Throwable) {
                    callRef.getAndSet(null)?.cancel()
                    if (continuation.isActive) {
                        if (error is IOException) onNetworkResult(false)
                        continuation.resumeWithException(error)
                    }
                } finally {
                    leaseAttachment?.close()
                    callRef.getAndSet(null)?.cancel()
                }
            }
            continuation.invokeOnCancellation {
                callRef.getAndSet(null)?.cancel()
                worker.cancel()
            }
        }
    }

    private fun validatedBaseUrl(): String {
        val value = baseUrl.trim().trimEnd('/')
        val uri = URI(value)
        val secure = uri.scheme == "https"
        val testLoopback = allowInsecureLoopbackForTests && uri.scheme == "http" &&
            uri.host in setOf("127.0.0.1", "localhost", "::1")
        require((secure || testLoopback) && !uri.host.isNullOrBlank() && uri.userInfo == null)
        require(uri.query == null && uri.fragment == null)
        return value
    }

    private fun InputStream?.readLimited(): String {
        if (this == null) return ""
        return buffered().use { input ->
            val result = ByteArray(MAX_RESPONSE_BYTES + 1)
            var offset = 0
            while (offset < result.size) {
                val read = input.read(result, offset, result.size - offset)
                if (read < 0) break
                offset += read
            }
            require(offset <= MAX_RESPONSE_BYTES) { "Account response is too large" }
            result.copyOf(offset).toString(Charsets.UTF_8)
        }
    }

    private companion object {
        val REQUEST_SCOPE = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        const val CONNECT_TIMEOUT_MS = 4_000
        const val READ_TIMEOUT_MS = 8_000
        const val CALL_TIMEOUT_MS = 10_000
        const val MAX_RESPONSE_BYTES = 1 shl 20
        val JSON_MEDIA_TYPE = "application/json; charset=utf-8".toMediaType()
        val HTTP = OkHttpClient.Builder()
            .retryOnConnectionFailure(false)
            .followRedirects(false)
            .followSslRedirects(false)
            .connectTimeout(CONNECT_TIMEOUT_MS.toLong(), TimeUnit.MILLISECONDS)
            .readTimeout(READ_TIMEOUT_MS.toLong(), TimeUnit.MILLISECONDS)
            .writeTimeout(READ_TIMEOUT_MS.toLong(), TimeUnit.MILLISECONDS)
            .callTimeout(CALL_TIMEOUT_MS.toLong(), TimeUnit.MILLISECONDS)
            .build()
    }
}
