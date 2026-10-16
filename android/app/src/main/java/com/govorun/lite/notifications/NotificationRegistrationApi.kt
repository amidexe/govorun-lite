package com.govorun.lite.notifications

import com.govorun.lite.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import java.io.IOException
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import okhttp3.Call
import okhttp3.Callback
import okhttp3.Response

internal class NotificationRegistrationApi {
    private val origin = BuildConfig.NOTIFICATIONS_API_URL.toHttpUrlOrNull()
    val configured: Boolean get() = origin?.let {
        it.isHttps && it.username.isEmpty() && it.password.isEmpty() && it.query == null && it.fragment == null
    } == true && BuildConfig.FIREBASE_APP_ID.isNotBlank() && BuildConfig.FIREBASE_API_KEY.isNotBlank()
    private val client = OkHttpClient.Builder().callTimeout(10, TimeUnit.SECONDS)
        .connectTimeout(5, TimeUnit.SECONDS).readTimeout(8, TimeUnit.SECONDS)
        .followRedirects(false).followSslRedirects(false).build()

    fun cancel() { client.dispatcher.cancelAll() }

    suspend fun sync(state: NotificationState) {
        check(configured)
        val wire = state.decision("request", extra = JSONObject().put("package", BuildConfig.APPLICATION_ID))
        val body = wire.getJSONObject("body")
        val request = Request.Builder().url(checkNotNull(origin).newBuilder()
            .encodedPath(wire.getString("path")).build())
            .header("Authorization", "Bearer ${state.credential}")
            .method(wire.getString("method"), body.toString().toRequestBody("application/json".toMediaType()))
            .build()
        suspendCancellableCoroutine<Unit> { continuation ->
            val call = client.newCall(request)
            continuation.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, error: IOException) { continuation.resumeWithException(error) }
                override fun onResponse(call: Call, response: Response) {
                    response.use {
                        if (it.code == 204) continuation.resume(Unit)
                        else continuation.resumeWithException(IOException("Notification registration unavailable"))
                    }
                }
            })
        }
    }
}
