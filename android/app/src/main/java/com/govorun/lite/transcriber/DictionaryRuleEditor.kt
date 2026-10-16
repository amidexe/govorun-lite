package com.govorun.lite.transcriber

import com.govorun.lite.shared.SharedDictionary
import org.json.JSONObject

/** Value-only JNI adapters for the common dictionary editor. */
internal object DictionaryRuleEditor {
    fun normalizeVoiceVariant(raw: String): String = SharedDictionary.evaluate(JSONObject()
        .put("kind", "normalize").put("text", raw)).getString("text")

    fun extractVoiceVariants(raw: String): List<String> = SharedDictionary.variants(JSONObject()
        .put("kind", "variants").put("text", raw))

    fun appendUniqueVoiceVariants(existing: List<String>, recognized: String, correctText: String): List<String> =
        SharedDictionary.variants(JSONObject().put("kind", "appendVariants")
            .put("existing", SharedDictionary.strings(existing)).put("recognized", recognized).put("correct", correctText))

    fun upsertVoiceMappings(raw: String, correctText: String, captured: List<String>): String =
        SharedDictionary.evaluate(JSONObject().put("kind", "voiceMappings").put("raw", raw)
            .put("correct", correctText).put("captured", SharedDictionary.strings(captured))).getString("raw")

    fun upsertTextMapping(raw: String, recognized: String, replacement: String): String =
        SharedDictionary.evaluate(JSONObject().put("kind", "textMapping").put("raw", raw)
            .put("recognized", recognized).put("replacement", replacement)).getString("raw")
}
