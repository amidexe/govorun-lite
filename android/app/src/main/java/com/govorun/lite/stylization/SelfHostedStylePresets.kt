package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.util.StorageKeys
import org.json.JSONArray
import org.json.JSONObject

/** Current local-only storage binding; Core owns category admission/mutations. */
internal object SelfHostedStylePresets {
    internal const val KEY_PRESETS = "stylization_self_hosted_presets_v1"
    const val MAX_PRESETS = 4
    const val MAX_NAME_CHARS = 24
    fun list(context: Context): List<CustomStylePreset> = runCatching {
        CustomStyleBinding.presets(decision(context, JSONObject()).getJSONArray("items"))
    }.getOrDefault(emptyList())
    fun save(context: Context, id: String?, name: String, instruction: String): CustomStylePreset? {
        val result = decision(context, JSONObject().put("id", id).put("newId", "own_${java.util.UUID.randomUUID()}")
            .put("name", name).put("instruction", instruction))
        val preset = result.optJSONObject("preset") ?: return null
        store(context, result.getJSONArray("items")); return CustomStyleBinding.preset(preset)
    }
    fun delete(context: Context, id: String) {
        store(context, decision(context, JSONObject().put("id", id).put("delete", true)).getJSONArray("items"))
    }
    private fun decision(context: Context, command: JSONObject): JSONObject = selfHostedDecision(command.put("kind", "presets")
        .put("items", JSONArray(preferences(context).getString(KEY_PRESETS, "[]"))))
    private fun store(context: Context, values: JSONArray) { preferences(context).edit().putString(KEY_PRESETS, values.toString()).apply() }
    private fun preferences(context: Context) = context.getSharedPreferences(StorageKeys.PREFS_FILE, Context.MODE_PRIVATE)
}
