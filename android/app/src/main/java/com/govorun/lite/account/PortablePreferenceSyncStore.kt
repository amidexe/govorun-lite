package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.shared.SharedBusiness
import com.govorun.lite.stylization.StylizationStylePreferences
import com.govorun.lite.util.Prefs
import java.security.MessageDigest
import org.json.JSONArray
import org.json.JSONObject

/** Stores raw UI intents and opaque Rust state; it does not merge preference fields. */
internal class PortablePreferenceSyncStore(context: Context) {
    private val appContext = context.applicationContext
    private val prefs = appContext.getSharedPreferences("govorun_preferences_journal_v1", Context.MODE_PRIVATE)
    private var applying = false

    @Synchronized
    fun enqueue(accountId: String, changes: PortablePreferenceChanges) {
        if (applying) return
        append(accountId, JSONObject().put("kind", "change")
            .put("local", local()).put("changes", changes.toCurrentJson()))
    }

    @Synchronized
    fun enqueueLocalValues(accountId: String) {
        append(accountId, JSONObject().put("kind", "replace").put("local", local()))
    }

    fun hasAccountHistory(accountId: String): Boolean = prefs.contains(key(accountId))

    @Synchronized
    fun hasPending(accountId: String): Boolean = load(accountId).let {
        it.getBoolean("hasPending") || it.getJSONArray("intents").length() > 0
    }

    @Synchronized
    fun step(accountId: String, flow: JSONObject?, event: JSONObject): JSONObject {
        val journal = load(accountId)
        val result = PreferenceSyncBinding.step(journal.get("state"), flow, local(),
            journal.getJSONArray("intents"), event)
        if (result.getString("action") == "complete" && !result.isNull("apply")) {
            applying = true
            try { apply(result.getJSONObject("apply")) } finally { applying = false }
        }
        val consumed = result.getInt("consumed")
        val intents = journal.getJSONArray("intents")
        check(consumed in 0..intents.length())
        val remaining = JSONArray((consumed until intents.length()).map(intents::getJSONObject))
        save(accountId, JSONObject().put("schemaVersion", 1).put("state", result.getJSONObject("state"))
            .put("hasPending", result.getBoolean("hasPending")).put("intents", remaining))
        return result
    }

    private fun local(): JSONObject {
        val options = StylizationStylePreferences.optionsForSelected(appContext)
        return JSONObject().put("defaultStyle", Prefs.getStylizationStyle(appContext))
            .put("punctuationMode", options.punctuation.apiId)
            .put("emojiMode", options.resolvedEmojiMode.apiId)
            .put("profanityFilterEnabled", options.profanityFilterEnabled)
    }

    private fun apply(value: JSONObject) {
        Prefs.setStylizationStyle(appContext, value.getString("defaultStyle"))
        val current = StylizationStylePreferences.optionsForSelected(appContext)
        StylizationStylePreferences.setSelectedOptions(appContext, current.copy(
            punctuation = PunctuationMode.valueOf(value.getString("punctuationMode").uppercase()),
            profanityFilterEnabled = value.getBoolean("profanityFilterEnabled"),
        ).withEmojiMode(EmojiMode.valueOf(value.getString("emojiMode").uppercase())))
    }

    private fun append(accountId: String, intent: JSONObject) {
        val journal = load(accountId)
        journal.getJSONArray("intents").put(intent)
        save(accountId, journal)
    }

    private fun load(accountId: String): JSONObject {
        val raw = prefs.getString(key(accountId), null) ?: return empty()
        return runCatching {
            JSONObject(raw).also { value ->
                require(value.getInt("schemaVersion") == 1)
                value.get("state"); value.getBoolean("hasPending"); value.getJSONArray("intents")
            }
        }.getOrElse { empty() }
    }
    private fun save(accountId: String, value: JSONObject) {
        check(prefs.edit().putString(key(accountId), value.toString()).commit())
    }
    private fun empty() = JSONObject().put("schemaVersion", 1).put("state", JSONObject.NULL)
        .put("hasPending", false).put("intents", JSONArray())
    private fun key(accountId: String): String = MessageDigest.getInstance("SHA-256")
        .digest(accountId.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }
}

internal object PreferenceSyncBinding {
    fun step(state: Any, flow: JSONObject?, local: JSONObject, intents: JSONArray,
             event: JSONObject): JSONObject = SharedBusiness.evaluate(
        JSONObject().put("operation", "preferencesSync").put("command", JSONObject()
            .put("state", state).put("flow", flow ?: JSONObject.NULL).put("local", local)
            .put("intents", intents).put("event", event)))
}

internal fun PortablePreferenceChanges.toCurrentJson() = JSONObject().also { json ->
    defaultStyle?.let { json.put("defaultStyle", it) }
    reducedPunctuation?.let { json.put("reducedPunctuation", it) }
    punctuationMode?.let { json.put("punctuationMode", it) }
    emojiEnabled?.let { json.put("emojiEnabled", it) }
    emojiMode?.let { json.put("emojiMode", it) }
    profanityFilterEnabled?.let { json.put("profanityFilterEnabled", it) }
    shortPhraseThreshold?.let { json.put("shortPhraseThreshold", it) }
}
