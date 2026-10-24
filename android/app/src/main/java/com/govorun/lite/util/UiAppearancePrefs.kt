package com.govorun.lite.util

import com.govorun.lite.model.settings.UiThemeMode
import android.content.Context
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import com.govorun.lite.storage.GovorunPreferences

data class UiAppearance(
    val themeMode: UiThemeMode = UiThemeMode.SYSTEM,
    // Off by default, as in DayPlan: the Govorun palette unless the user opts in.
    val dynamicColor: Boolean = false,
)

/**
 * Small application-scoped state holder for the Compose theme.
 *
 * Persistence remains outside composables, while every open activity observes
 * one state and reacts immediately when the user changes the appearance.
 */
object UiAppearancePrefs {
    private const val FILE_NAME = "govorun_ui_v2"
    private const val KEY_THEME_MODE = "theme_mode"
    private const val KEY_DYNAMIC_COLOR = "dynamic_color_enabled"

    private val mutableAppearance = mutableStateOf(UiAppearance())
    val appearance: State<UiAppearance> = mutableAppearance

    fun initialize(context: Context) {
        mutableAppearance.value = read(context)
    }

    fun read(context: Context): UiAppearance {
        val prefs = GovorunPreferences.of(context, FILE_NAME)
        return UiAppearance(
            themeMode = UiThemeMode.fromId(prefs.getString(KEY_THEME_MODE, null)),
            dynamicColor = prefs.getBoolean(KEY_DYNAMIC_COLOR, false),
        )
    }

    fun setThemeMode(context: Context, mode: UiThemeMode) {
        GovorunPreferences.of(context, FILE_NAME)
            .edit()
            .putString(KEY_THEME_MODE, mode.id)
            .apply()
        mutableAppearance.value = mutableAppearance.value.copy(themeMode = mode)
    }

    fun setDynamicColor(context: Context, enabled: Boolean) {
        GovorunPreferences.of(context, FILE_NAME)
            .edit()
            .putBoolean(KEY_DYNAMIC_COLOR, enabled)
            .apply()
        mutableAppearance.value = mutableAppearance.value.copy(dynamicColor = enabled)
    }

    fun reset(context: Context) {
        GovorunPreferences.of(context, FILE_NAME)
            .edit()
            .clear()
            .apply()
        mutableAppearance.value = UiAppearance()
    }
}
