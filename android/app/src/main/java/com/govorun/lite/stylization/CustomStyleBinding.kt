package com.govorun.lite.stylization

import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import org.json.JSONArray
import org.json.JSONObject

/** Current V1 keys and DTO conversion. No trimming, limits, selection or mutation policy. */
internal object CustomStyleBinding {
    fun presets(array: JSONArray): List<CustomStylePreset> = (0 until array.length()).map { preset(array.getJSONObject(it)) }
    /** Every saved style that can be read; one broken entry never hides the others. */
    fun readablePresets(raw: String?): List<CustomStylePreset> {
        val array = runCatching { JSONArray(raw ?: "[]") }.getOrElse { return emptyList() }
        return (0 until array.length()).mapNotNull { index -> runCatching { preset(array.getJSONObject(index)) }.getOrNull() }
    }
    fun preset(item: JSONObject): CustomStylePreset {
        for (key in listOf("id", "name", "instruction")) require(item.get(key) is String)
        val enabled = item.optBoolean("emojiEnabled", false)
        return CustomStylePreset(item.getString("id"), item.getString("name"), item.getString("instruction"),
            PunctuationMode.fromApiId(item.optString("punctuationMode")), enabled,
            item.optBoolean("profanityFilterEnabled", false), EmojiMode.fromApiId(item.optString("emojiMode"), enabled))
    }
    fun encode(presets: List<CustomStylePreset>): JSONArray = JSONArray().apply {
        presets.forEach { preset -> put(JSONObject().put("id", preset.id).put("name", preset.name)
            .put("instruction", preset.instruction).put("punctuationMode", preset.punctuationMode.apiId)
            .put("emojiEnabled", preset.resolvedEmojiMode != EmojiMode.OFF)
            .put("emojiMode", preset.resolvedEmojiMode.apiId).put("profanityFilterEnabled", preset.profanityFilterEnabled)) }
    }
    fun options(value: StyleOptions): JSONObject = JSONObject().put("punctuationMode", value.punctuation.apiId)
        .put("emojiMode", value.resolvedEmojiMode.apiId).put("profanityFilterEnabled", value.profanityFilterEnabled)
}
