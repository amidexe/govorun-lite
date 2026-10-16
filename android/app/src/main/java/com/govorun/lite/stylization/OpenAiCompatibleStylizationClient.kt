package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.StylizationResponse
import com.govorun.lite.model.stylization.TextStyle
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import org.json.JSONArray
import org.json.JSONObject

internal class OpenAiCompatibleStylizationClient(
    private val settings: SelfHostedServerSettings,
) : StylizationClient {
    override suspend fun stylize(
        text: String,
        style: TextStyle,
        inputMode: StylizationInputMode,
        instruction: String?,
        punctuationMode: PunctuationMode,
        emojiEnabled: Boolean,
        profanityFilterEnabled: Boolean,
        emojiMode: EmojiMode,
        preparationMillis: Long,
    ): StylizationResponse {
        check(settings.hasConsent) { "Подтвердите передачу данных своему серверу" }
        require(text.isNotBlank())
        val prompt = instruction.orEmpty().trim()
        require(prompt.length >= 3)
        val endpoint = settings.normalizedEndpoint ?: error("Invalid self-hosted endpoint")
        require(settings.model.isNotBlank())
        val request = Request.Builder().url(endpoint)
            .header("Accept", "application/json")
            .post(buildRequest(settings.model, prompt, text).toString()
                .toRequestBody("application/json; charset=utf-8".toMediaType()))
            .apply {
                if (settings.apiKey.isNotBlank()) header("Authorization", "Bearer ${settings.apiKey}")
            }.build()
        return suspendCancellableCoroutine { continuation ->
            val client = if (settings.proxyEnabled) http.newBuilder()
                .proxy(SelfHostedProxy.route(true, settings.proxyURL)).build() else http
            val call = client.newCall(request)
            continuation.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) {
                    continuation.resumeWith(Result.failure(e))
                }
                override fun onResponse(call: Call, response: Response) {
                    val result = runCatching {
                        response.use {
                            if (!it.isSuccessful) throw StylizationHttpException(it.code)
                            val styled = parseResponse(it.body.string())
                            require(styled.isNotEmpty())
                            StylizationResponse(styled)
                        }
                    }
                    continuation.resumeWith(result)
                }
            })
        }
    }

    internal companion object {
        // User-managed API: no Govorun deadlines, size quotas or automatic paid retries.
        internal val http = OkHttpClient.Builder()
            .callTimeout(0, TimeUnit.MILLISECONDS).connectTimeout(0, TimeUnit.MILLISECONDS)
            .readTimeout(0, TimeUnit.MILLISECONDS).writeTimeout(0, TimeUnit.MILLISECONDS)
            .retryOnConnectionFailure(false).followRedirects(false).build()

        fun buildRequest(model: String, instruction: String, text: String): JSONObject =
            JSONObject().put("model", model.trim()).put("messages", JSONArray()
                .put(JSONObject().put("role", "system").put("content", instruction))
                .put(JSONObject().put("role", "user").put("content", text)))
                .put("stream", false)

        fun parseResponse(raw: String): String {
            val content = JSONObject(raw).optJSONArray("choices")?.optJSONObject(0)
                ?.optJSONObject("message")?.opt("content")
            return (content as? String)?.trim().orEmpty()
        }
    }
}
