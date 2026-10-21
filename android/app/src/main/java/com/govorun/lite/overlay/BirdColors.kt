package com.govorun.lite.overlay

import android.content.Context
import androidx.core.content.ContextCompat
import com.google.android.material.color.MaterialColors

/**
 * Colours the floating bird can take: [SYSTEM] follows the phone's wallpaper
 * palette, as the first version did, and each preset is a seed. Each gives one
 * saturated Material accent (the light-scheme tone, kept on dark screens too so
 * red stays red) and the colour drawn on it; idle and recording differ only in
 * their transparency settings.
 */
internal object BirdColors {
    const val SYSTEM = "system"
    const val RED = "red"
    const val DEFAULT = RED

    data class Preset(val id: String, val label: String, val seed: Int)

    /** Material accent role and its on-colour. */
    data class Tone(val fill: Int, val onFill: Int)

    val presets = listOf(
        Preset(RED, "Красный", 0xFFE53935.toInt()),
        Preset("orange", "Оранжевый", 0xFFFB8C00.toInt()),
        Preset("green", "Зелёный", 0xFF43A047.toInt()),
        Preset("teal", "Бирюзовый", 0xFF00897B.toInt()),
        Preset("blue", "Синий", 0xFF1E88E5.toInt()),
        Preset("purple", "Фиолетовый", 0xFF8E24AA.toInt()),
        Preset("pink", "Розовый", 0xFFD81B60.toInt()),
        Preset("grey", "Серый", 0xFF757575.toInt()),
    )

    fun preset(id: String): Preset? = presets.firstOrNull { it.id == id }

    val ids: List<String> = listOf(SYSTEM) + presets.map { it.id }

    fun normalize(id: String?): String = if (id != null && id in ids) id else DEFAULT

    fun label(id: String): String = preset(id)?.label ?: "Как в системе"

    fun tone(context: Context, id: String): Tone {
        val preset = preset(id) ?: return Tone(
            // The wallpaper palette's primary/onPrimary, as Android maps it in light.
            ContextCompat.getColor(context, android.R.color.system_accent1_600),
            ContextCompat.getColor(context, android.R.color.system_accent1_0),
        )
        val roles = MaterialColors.getColorRoles(preset.seed, true)
        return Tone(roles.accent, roles.onAccent)
    }
}
