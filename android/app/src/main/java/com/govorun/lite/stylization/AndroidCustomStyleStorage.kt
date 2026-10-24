package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.storage.GovorunPreferences
import com.govorun.lite.util.StorageKeys
import com.govorun.lite.util.Prefs
import org.json.JSONArray
import org.json.JSONObject

/** Atomic current-key persistence. No normalization, truncation or library rules. */
internal class AndroidCustomStyleStorage(context: Context) : CustomStyleStorage {
    private val app = context.applicationContext
    private val preferences = GovorunPreferences.of(app, StorageKeys.PREFS_FILE)
    override val lock: Any get() = libraryLock
    val activeId: String? get() = preferences.getString(KEY_ACTIVE_ID, null)
    fun list(): List<CustomStylePreset> = synchronized(lock) { CustomStyleBinding.readablePresets(preferences.getString(KEY_PRESETS, "[]")) }
    /** Strict: a catalog the library cannot read whole is never rewritten by an edit. */
    override fun read(): JSONObject = synchronized(lock) {
        val raw = try { CustomStyleBinding.presets(JSONArray(preferences.getString(KEY_PRESETS, "[]"))) }
            catch (_: Exception) { throw IllegalArgumentException("style_storage_unavailable") }
        state(raw)
    }
    /** What dictation reads: every saved style that is readable. */
    fun readable(): JSONObject = synchronized(lock) { state(list()) }
    private fun state(presets: List<CustomStylePreset>) = JSONObject().put("presets", CustomStyleBinding.encode(presets))
        .put("activeId", activeId ?: JSONObject.NULL)
        .put("selectedStyle", Prefs.getStylizationStyle(app)).put("instruction", Prefs.getStylizationCustomInstruction(app))
    override fun apply(before: JSONObject, after: JSONObject) {
        val editor = preferences.edit()
        if (CustomStyleBinding.presets(before.getJSONArray("presets")) != CustomStyleBinding.presets(after.getJSONArray("presets"))) {
            editor.putString(KEY_PRESETS, after.getJSONArray("presets").toString())
        }
        if (before.get("activeId") != after.get("activeId")) {
            editor.putString(KEY_ACTIVE_ID, if (after.isNull("activeId")) null else after.getString("activeId"))
        }
        if (before.getString("selectedStyle") != after.getString("selectedStyle")) {
            editor.putString(StorageKeys.Key.STYLIZATION_STYLE, after.getString("selectedStyle"))
        }
        if (before.getString("instruction") != after.getString("instruction")) {
            editor.putString("stylization_custom_instruction", after.getString("instruction"))
        }
        check(editor.commit()) { "style_storage_write_failed" }
    }
    override fun selectedOptions(): StyleOptions = StylizationStylePreferences.optionsForSelected(app)
    override fun newId(): String = "custom_${java.util.UUID.randomUUID()}"
    private companion object {
        val libraryLock = Any()
        const val KEY_PRESETS = "stylization_custom_presets_v1"
        const val KEY_ACTIVE_ID = "stylization_active_custom_preset_v1"
    }
}
