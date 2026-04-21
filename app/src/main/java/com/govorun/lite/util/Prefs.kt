package com.govorun.lite.util

import android.content.Context

/**
 * Single place that knows the app's SharedPreferences file name + keys.
 * Keeps key spelling consistent across UI (where the user toggles it) and
 * the AccessibilityService (which reads it on every bubble tap).
 */
object Prefs {

    private const val FILE_NAME = "govorun_lite_prefs"
    private const val KEY_HAPTICS_ENABLED = "haptics_enabled"
    private const val KEY_BUBBLE_ALPHA = "bubble_alpha"

    // Clamp range for the bubble fill alpha. The floor (0.4) keeps the
    // bubble visible enough that the bird silhouette is still recognisable
    // on any wallpaper — allowing lower values leads to "where did it go?"
    // support questions. Ceiling 1.0 is fully opaque.
    const val BUBBLE_ALPHA_MIN = 0.4f
    const val BUBBLE_ALPHA_MAX = 1.0f
    const val BUBBLE_ALPHA_STEP = 0.05f
    // Slight translucency by default — enough to blend the bubble with a
    // patterned wallpaper, not so much that it becomes hard to spot. Must
    // land on the slider step (0.4 + N × 0.05) or the M3 Slider throws
    // IllegalStateException on setValue.
    const val BUBBLE_ALPHA_DEFAULT = 0.85f

    fun isHapticsEnabled(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_HAPTICS_ENABLED, false)

    fun setHapticsEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_HAPTICS_ENABLED, enabled)
            .apply()
    }

    fun getBubbleAlpha(context: Context): Float {
        val raw = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getFloat(KEY_BUBBLE_ALPHA, BUBBLE_ALPHA_DEFAULT)
        return snapBubbleAlpha(raw)
    }

    fun setBubbleAlpha(context: Context, alpha: Float) {
        val snapped = snapBubbleAlpha(alpha)
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putFloat(KEY_BUBBLE_ALPHA, snapped)
            .apply()
    }

    // Always land on a multiple of the slider step — M3 Slider throws if
    // setValue() is called with anything between two steps, so we keep the
    // pref storage aligned too. Rounded to 2 decimal places to neutralise
    // float drift (0.4f + 9 * 0.05f = 0.8500001f, which would squeak past
    // the current Slider tolerance but could break in a future release).
    private fun snapBubbleAlpha(value: Float): Float {
        val clamped = value.coerceIn(BUBBLE_ALPHA_MIN, BUBBLE_ALPHA_MAX)
        val steps = Math.round((clamped - BUBBLE_ALPHA_MIN) / BUBBLE_ALPHA_STEP)
        val raw = BUBBLE_ALPHA_MIN + steps * BUBBLE_ALPHA_STEP
        val rounded = Math.round(raw * 100f) / 100f
        return rounded.coerceIn(BUBBLE_ALPHA_MIN, BUBBLE_ALPHA_MAX)
    }
}
