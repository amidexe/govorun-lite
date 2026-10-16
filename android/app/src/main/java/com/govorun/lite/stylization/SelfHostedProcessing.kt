package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.StylizationResponse
import com.govorun.lite.model.stylization.StylizationResult
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import com.govorun.lite.shared.SharedDictationRules
import org.json.JSONObject

/** A single user-managed request, with local-only, content-free diagnostics. */
internal object SelfHostedProcessing {
    suspend fun process(
        text: String,
        record: (SelfHostedDiagnostic) -> Unit,
        request: suspend () -> StylizationResponse,
    ): StylizationResult {
        val started = System.nanoTime()
        var outcome = "failed"
        try {
            val begin = SharedDictationRules.selfHosted(JSONObject().put("kind", "begin")
                .put("input", SharedDictationRules.units(text)))
            if (begin.getString("action") == "local") { outcome = ""; return StylizationResult(text) }
            check(begin.getString("action") == "request")
            val response = request()
            currentCoroutineContext().ensureActive()
            val decision = SharedDictationRules.selfHosted(JSONObject().put("kind", "response")
                .put("input", SharedDictationRules.units(text)).put("output", SharedDictationRules.units(response.text)))
            check(decision.getString("action") in setOf("local", "output"))
            outcome = decision.getString("diagnostic")
            return StylizationResult(if (decision.getString("action") == "output") response.text else text,
                usedFallback = decision.getBoolean("fallback"),
                fallbackReason = if (decision.getBoolean("fallback"))
                    com.govorun.lite.model.stylization.StylizationFallbackReason.INVALID_RESPONSE else null)
        } catch (error: CancellationException) {
            outcome = "cancelled"
            throw error
        } catch (error: Exception) {
            outcome = runCatching { SharedDictationRules.selfHosted(JSONObject().put("kind", "failure")
                .put("status", (error as? StylizationHttpException)?.statusCode ?: 0))
                .getString("diagnostic") }.getOrDefault("failed")
            return StylizationResult(text, usedFallback = true, fallbackReason = requestFailureReason(error))
        } finally {
            if (outcome.isNotEmpty()) runCatching { record(SelfHostedDiagnostic(outcome, (System.nanoTime() - started) / 1_000_000)) }
        }
    }
}

internal data class SelfHostedDiagnostic(val result: String, val elapsedMs: Long) {
    val success get() = result == "success" || result == "unchanged"
    val title get() = when (result) {
        "success" -> "Свой сервер: текст обработан"
        "unchanged" -> "Свой сервер: текст без изменений"
        "auth_failed" -> "Свой сервер: проверьте ключ и доступ к модели"
        "cancelled" -> "Свой сервер: запрос отменён"
        else -> "Свой сервер: ошибка, использован исходный текст"
    }
    fun encode() = "self_hosted result=$result elapsed_ms=${elapsedMs.coerceIn(0, 999_999_999)}"
    companion object {
        fun parse(value: String?): SelfHostedDiagnostic? = diagnosticTiming(value)?.takeUnless { it.has("phase") }
            ?.let { SelfHostedDiagnostic(it.getString("result"), it.getString("elapsed_ms").toLong()) }
    }
}
