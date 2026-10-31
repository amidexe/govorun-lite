package com.govorun.lite.api.smart

import com.govorun.lite.model.stylization.StylizationFallbackReason
import com.govorun.lite.model.stylization.StylizationResult
import com.govorun.lite.model.stylization.StylizationSkipReason
import com.govorun.lite.stylization.titleRes
import org.json.JSONObject

/**
 * «Улучшить через Говорун» for Echo: a note's text through Govorun's own text improvement, the same path, limits, consent, «Офлайн»
 * and length rule as a dictation, with the style the person set for Echo in «Стили для приложений». The answer is the improved
 * text, or why there is none, in words Echo can act on.
 */
internal object SmartImprove {
    const val IMPROVE_OFF = "improve_off"
    const val TOO_SHORT = "too_short"
    const val TOO_LONG = "too_long"
    const val APP_RULE_OFF = "app_rule_off"
    const val STYLIZATION_TERMS = "stylization_terms"

    /** Echo's allowance for the whole period is spent: no use asking again tomorrow, unlike [SmartState.LIMIT_TODAY]. */
    const val LIMIT_PERIOD = "limit_period"

    /**
     * `{"text": …}` when it was improved (or there was nothing to change); otherwise `{"code": …}`. [periodSpent]: Echo's period is
     * used up. [refusal]: the person's own «Офлайн» or roaming choice, which Govorun's «no network» does not tell apart.
     * [signedIn]: without an account the core finds no access; Echo should ask to sign in, not for access.
     */
    fun answer(
        result: StylizationResult,
        periodSpent: Boolean = false,
        refusal: com.govorun.lite.network.NetworkRefusal? = null,
        signedIn: Boolean = true,
    ): JSONObject {
        val skipped = result.skipReason
        val fallback = result.fallbackReason
        val code =
            when {
                skipped != null ->
                    when (skipped) {
                        StylizationSkipReason.STYLIZATION_DISABLED -> IMPROVE_OFF
                        StylizationSkipReason.ACCESS_INACTIVE -> if (signedIn) SmartState.NEEDS_ACCESS.wire else SmartState.NOT_SIGNED_IN.wire
                        StylizationSkipReason.SHORT_PHRASE -> TOO_SHORT
                        StylizationSkipReason.APP_RULE_DISABLED -> APP_RULE_OFF
                        else -> SmartState.UNAVAILABLE.wire
                    }
                !result.usedFallback || fallback == StylizationFallbackReason.NO_CHANGE -> null
                else ->
                    when (fallback) {
                        StylizationFallbackReason.TERMS_UPDATE_REQUIRED -> STYLIZATION_TERMS
                        StylizationFallbackReason.TEXT_TOO_LONG -> TOO_LONG
                        // Only the person's choice waits for the person; a network that is simply gone is tried again by
                        // Echo's queue by itself.
                        StylizationFallbackReason.OFFLINE -> when (refusal) {
                            com.govorun.lite.network.NetworkRefusal.OFFLINE -> SmartState.OFFLINE.wire
                            com.govorun.lite.network.NetworkRefusal.ROAMING -> SmartState.ROAMING.wire
                            null -> SmartState.NO_NETWORK.wire
                        }
                        StylizationFallbackReason.STYLE_PASS_RESERVE_EMPTY -> if (periodSpent) LIMIT_PERIOD else SmartState.LIMIT_TODAY.wire
                        StylizationFallbackReason.NOT_CONFIGURED, StylizationFallbackReason.SERVICE_UNAVAILABLE -> SmartState.UNAVAILABLE.wire
                        StylizationFallbackReason.SERVER_TIMEOUT -> SmartApi.TIMEOUT
                        StylizationFallbackReason.SERVER_BUSY -> SmartApi.BUSY
                        else -> SmartApi.SERVER_ERROR
                    }
            }
        return if (code == null) JSONObject().put("text", result.text) else JSONObject().put("code", code)
    }

    /**
     * The styles Echo may offer for a note: Govorun's own three and the person's styles, each with Govorun's name for it, in
     * Govorun's order. The value goes back with the text.
     */
    fun styles(context: android.content.Context): org.json.JSONArray {
        val array = org.json.JSONArray()
        // «Заметка» first: a name, a description and the text laid out, as only Govorun's own server makes it.
        array.put(JSONObject().put("value", com.govorun.lite.stylization.EchoNoteMark.NOTE_STYLE).put("title", context.getString(com.govorun.lite.R.string.echo_note_style)))
        listOf(com.govorun.lite.model.stylization.TextStyle.LIGHT, com.govorun.lite.model.stylization.TextStyle.FORMAL, com.govorun.lite.model.stylization.TextStyle.CONCISE).forEach {
            array.put(JSONObject().put("value", it.apiId).put("title", context.getString(it.titleRes)))
        }
        runCatching { com.govorun.lite.stylization.AndroidCustomStyles.create(context).list() }.getOrDefault(emptyList()).forEach {
            array.put(JSONObject().put("value", "custom:${it.id}").put("title", it.name))
        }
        return array
    }
}
