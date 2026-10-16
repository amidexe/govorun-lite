package com.govorun.lite.stylization

import android.content.Context
import com.govorun.lite.model.stylization.CustomStylePreset

/** Read/sync facade has no outbound account effects; local edits use AndroidCustomStyles. */
internal object CustomStylePresets {
    val MAX_PRESETS: Int get() = CustomStylePolicy.limits.getInt("maxPresets")
    val MAX_NAME_CHARS: Int get() = CustomStylePolicy.limits.getInt("maxNameCharacters")
    private fun local(context: Context) = AndroidCustomStyleLibrary(AndroidCustomStyleStorage(context), beforeChange = {})
    fun list(context: Context): List<CustomStylePreset> = local(context).list()
    fun activeId(context: Context): String? = local(context).activeId()
    fun canCreate(context: Context): Boolean = local(context).snapshot()
        ?.getJSONObject("projection")?.getBoolean("canCreatePreset") ?: false
    fun replaceFromSync(context: Context, presets: List<CustomStylePreset>, selectedID: String) =
        local(context).replaceFromSync(presets, selectedID)
}
