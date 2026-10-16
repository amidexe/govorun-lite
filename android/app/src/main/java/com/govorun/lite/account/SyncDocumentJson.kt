package com.govorun.lite.account

import org.json.JSONArray
import org.json.JSONObject

internal object SyncDocumentJson {
    fun encode(document: SyncedContentDocument): JSONObject = JSONObject()
        .put("schemaVersion", document.schemaVersion)
        .put(
            "dictionary",
            JSONObject()
                .put("enabled", document.dictionary.enabled)
                .put("text", document.dictionary.text),
        )
        .put("selectedCustomStyleId", document.selectedCustomStyleId)
        .put(
            "customStyles",
            JSONArray().apply {
                document.customStyles.forEach { style ->
                    put(
                        JSONObject()
                            .put("id", style.id)
                            .put("name", style.name)
                            .put("instruction", style.instruction)
                            .put("punctuationMode", style.punctuationMode)
                            .put("emojiEnabled", style.emojiEnabled)
                            .put("emojiMode", style.emojiMode)
                            .put("profanityFilterEnabled", style.profanityFilterEnabled),
                    )
                }
            },
        )

    fun decode(raw: String): SyncedContentDocument = decode(JSONObject(raw))

    fun decodeResponse(raw: String, etag: String?, status: Int = 200, fallback: String = ""): SyncDocumentResponse {
        val json = com.govorun.lite.shared.SharedBusiness.evaluate(JSONObject().put("operation", "contentSync")
            .put("command", JSONObject().put("kind", "readResponse").put("body", raw).put("status", status)
                .put("etag", etag).put("fallback", fallback)))
        return SyncDocumentResponse(json.getBoolean("configured"), json.getString("etag"),
            decode(json.getJSONObject("document")), json.getBoolean("notModified"))
    }

    private fun decode(json: JSONObject): SyncedContentDocument {
        val schema = json.getInt("schemaVersion")
        val dictionary = json.getJSONObject("dictionary")
        val text = dictionary.getString("text")
        val styles = json.getJSONArray("customStyles")
        return SyncedContentDocument(
            schemaVersion = schema,
            dictionary = SyncedDictionary(dictionary.getBoolean("enabled"), text),
            selectedCustomStyleId = json.getString("selectedCustomStyleId"),
            customStyles = List(styles.length()) { index ->
                styles.getJSONObject(index).let { value ->
                    SyncedCustomStyle(
                        id = value.getString("id"),
                        name = value.getString("name"),
                        instruction = value.getString("instruction"),
                        punctuationMode = value.getString("punctuationMode"),
                        emojiEnabled = value.getBoolean("emojiEnabled"),
                        emojiMode = value.getString("emojiMode"),
                        profanityFilterEnabled = value.getBoolean("profanityFilterEnabled"),
                    )
                }
            },
        ).also(::requireValid)
    }

    fun requireValid(document: SyncedContentDocument) = requireValidSyncedContent(document)

}
