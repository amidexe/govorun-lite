package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.BuildConfig
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.officialStylizationActive
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StylizationClientOutcome
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.StylizationResponse
import com.govorun.lite.model.stylization.TextStyle

internal interface StylizationClient {
    suspend fun stylize(
        text: String,
        style: TextStyle,
        inputMode: StylizationInputMode,
        instruction: String? = null,
        punctuationMode: PunctuationMode = PunctuationMode.NORMAL,
        emojiEnabled: Boolean = false,
        profanityFilterEnabled: Boolean = false,
        emojiMode: EmojiMode = EmojiMode.fromApiId(null, emojiEnabled),
        preparationMillis: Long = 0,
    ): StylizationResponse

    suspend fun reportClientOutcome(
        outcomePass: String,
        outcome: StylizationClientOutcome,
        latencyBucket: String,
    ) = Unit
}

/** Current Promo transport. The removed shared-token endpoint is not a
 * fallback: all online text uses anonymous one-use passes and the official
 * provider disclosure. */
internal class ConfiguredStylizationClient(
    @Suppress("UNUSED_PARAMETER") appContext: Context,
) : StylizationClient {
    override suspend fun stylize(
        text: String, style: TextStyle, inputMode: StylizationInputMode, instruction: String?,
        punctuationMode: PunctuationMode, emojiEnabled: Boolean, profanityFilterEnabled: Boolean,
        emojiMode: EmojiMode, preparationMillis: Long,
    ): StylizationResponse {
        val payload = org.json.JSONObject().put("text", text).put("style", style.apiId)
            .put("inputMode", inputMode.apiId).put("punctuationMode", punctuationMode.apiId)
            .put("reducedPunctuation", punctuationMode == PunctuationMode.LESS)
            .put("emojiEnabled", emojiEnabled).put("emojiMode", emojiMode.apiId)
            .put("profanityFilterEnabled", profanityFilterEnabled)
        if (instruction != null) payload.put("instruction", instruction)
        val connectionId = kotlinx.coroutines.currentCoroutineContext()[PreparedStylesConnection]?.requestId()
        val result = GovorunAccountRepository.sharedAccountCommand(org.json.JSONObject()
            .put("operation", "stylizationProcess")
            .put("sessionGeneration", GovorunAccountRepository.state.value.sessionGeneration)
            .put("preparationMillis", preparationMillis).put("payload", payload)
            .put("requestId", connectionId ?: org.json.JSONObject.NULL))
        return StylizationResponse(result.getString("text"),
            result.optString("outcomePass").takeUnless { result.isNull("outcomePass") },
            result.getString("latencyBucket"))
    }
    override suspend fun reportClientOutcome(outcomePass: String, outcome: StylizationClientOutcome, latencyBucket: String) {
        GovorunAccountRepository.sharedAccountCommand(org.json.JSONObject()
            .put("operation", "stylizationOutcome")
            .put("sessionGeneration", GovorunAccountRepository.state.value.sessionGeneration)
            .put("pass", outcomePass).put("outcome", outcome.apiId).put("latencyBucket", latencyBucket))
    }
}

internal class StylizationHttpException(
    val statusCode: Int,
    val responseBody: String = "",
    val retryAfterMillis: Long? = null,
) :
    Exception("Stylization HTTP $statusCode")
