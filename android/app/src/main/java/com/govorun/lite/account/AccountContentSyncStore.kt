package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.util.Prefs
import org.json.JSONObject

/** IO only: current documents and an opaque shared workflow journal. No legacy imports. */
internal class AccountContentSyncStore(context: Context) {
    private val appContext = context.applicationContext
    private val preferences = appContext.getSharedPreferences("govorun_content_journal_v1", Context.MODE_PRIVATE)
    private var applying = false

    @Synchronized
    fun localDocument(): SyncedContentDocument {
        val styles = CustomStylePresets.list(appContext)
        return SyncedContentDocument(
        dictionary = SyncedDictionary(
            enabled = Prefs.isDictionaryEnabled(appContext),
            text = Prefs.getDictionary(appContext),
        ),
        selectedCustomStyleId = if (
            TextStyle.fromApiId(Prefs.getStylizationStyle(appContext)) == TextStyle.CUSTOM
        ) {
            CustomStylePresets.activeId(appContext).orEmpty()
        } else {
            ""
        },
        customStyles = styles.map { preset ->
            SyncedCustomStyle(
                id = preset.id,
                name = preset.name,
                instruction = preset.instruction,
                punctuationMode = preset.punctuationMode.apiId,
                emojiEnabled = preset.emojiEnabled,
                emojiMode = preset.resolvedEmojiMode.apiId,
                profanityFilterEnabled = preset.profanityFilterEnabled,
            )
        },
        ).also(SyncDocumentJson::requireValid)
    }

    fun hasAccountHistory(accountID: String): Boolean = preferences.contains(accountID)

    /** Same IO lock as a sync checkpoint. A reader cannot acknowledge a dirty
     * revision between its durable marker and the following local content write. */
    @Synchronized
    fun <T> withLocalMutation(action: () -> T): T = action()

    @Synchronized
    fun markLocalChange(@Suppress("UNUSED_PARAMETER") accountID: String?) {
        if (applying) return
        check(preferences.edit().putLong("revision", preferences.getLong("revision", 0) + 1).commit())
    }

    @Synchronized
    fun hasPending(accountID: String): Boolean {
        val entry = load(accountID)
        return entry?.getBoolean("hasPending") == true ||
            preferences.getLong("revision", 0) != (entry?.getLong("revision") ?: 0)
    }

    @Synchronized
    fun step(accountID: String, flow: JSONObject?, event: JSONObject): JSONObject {
        val revision = preferences.getLong("revision", 0)
        val result = ContentWorkflowBinding.step(load(accountID)?.get("state") ?: JSONObject.NULL,
            flow, localDocument(), event)
        result.optJSONObject("apply")?.let {
            applying = true
            try { applyRemote(SyncDocumentJson.decode(it.toString())) } finally { applying = false }
        }
        val entry = JSONObject().put("schemaVersion", 1).put("revision", revision)
            .put("state", result.getJSONObject("state")).put("hasPending", result.getBoolean("hasPending"))
        check(preferences.edit().putString(accountID, entry.toString()).commit())
        return result
    }

    private fun load(accountID: String): JSONObject? = preferences.getString(accountID, null)?.let {
        JSONObject(it).also { value -> check(value.getInt("schemaVersion") == 1) }
    }

    private fun applyRemote(document: SyncedContentDocument) {
        Prefs.setDictionaryEnabled(appContext, document.dictionary.enabled)
        Prefs.setDictionary(appContext, document.dictionary.text)
        CustomStylePresets.replaceFromSync(
            appContext,
            document.customStyles.map { style ->
                CustomStylePreset(
                    id = style.id,
                    name = style.name,
                    instruction = style.instruction,
                    punctuationMode = PunctuationMode.fromApiId(style.punctuationMode),
                    emojiEnabled = style.emojiEnabled,
                    emojiMode = EmojiMode.fromApiId(style.emojiMode, style.emojiEnabled),
                    profanityFilterEnabled = style.profanityFilterEnabled,
                )
            },
            document.selectedCustomStyleId,
        )
    }

}
