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
    private const val KEY_HIDE_IN_SEARCH = "hide_in_search"
    private const val KEY_BUBBLE_ALPHA = "bubble_alpha"
    private const val KEY_BUBBLE_SIZE = "bubble_size"
    private const val KEY_BUBBLE_Y = "bubble_y"
    private const val KEY_BUBBLE_SIDE = "bubble_side"
    // Version-suffixed so each release with new feature highlights re-shows
    // the card to existing users (their dismissal of the previous version's
    // card doesn't carry over). Bump the suffix when the card content changes.
    private const val KEY_WHATS_NEW_HINT_DISMISSED = "whats_new_hint_dismissed_v1010"
    private const val KEY_DICTIONARY = "dictionary_text"
    private const val KEY_DICTIONARY_ENABLED = "dictionary_enabled"
    private const val KEY_PAUSE_LENGTH = "pause_length"
    // One-shot flag: marks that we've already run the 1.0.8 migration that
    // bumps existing users from Short → Medium. Without this we'd reset
    // the pref on every cold start, which would override the user's
    // post-migration choice if they switched back to Short on purpose.
    private const val KEY_V108_PAUSE_MIGRATED = "v108_pause_migrated"

    const val BUBBLE_SIDE_RIGHT = "right"
    const val BUBBLE_SIDE_LEFT = "left"

    // VAD silence threshold presets — how long the silence must last before
    // the VAD considers the utterance finished. SHORT is the historic default
    // and matches what 1.0.6 and earlier shipped with; existing users keep it.
    // LONG gives "thinking pauses" room before the model commits a paragraph
    // — this is what the RuStore feedback ("she pauses to think and the app
    // cuts the sentence") asked for.
    const val PAUSE_SHORT = "short"
    const val PAUSE_MEDIUM = "medium"
    const val PAUSE_LONG = "long"
    const val PAUSE_DEFAULT = PAUSE_SHORT

    // Seconds, fed verbatim into SileroVadModelConfig.minSilenceDuration.
    // Doubling per step keeps the difference perceptually obvious.
    const val PAUSE_SHORT_SECONDS = 0.5f
    const val PAUSE_MEDIUM_SECONDS = 1.0f
    const val PAUSE_LONG_SECONDS = 2.0f

    // Clamp range for the bubble fill alpha. Floor 0.0 = fully transparent
    // background, only the bird silhouette visible. When idleAlphaFraction
    // approaches zero, BubbleView smoothly switches the bird tint toward
    // colorOnSurface — the MD3 "always readable on system surface" colour
    // that follows light/dark theme — so the outline stays visible on any
    // wallpaper or app underneath. Ceiling 1.0 is fully opaque.
    const val BUBBLE_ALPHA_MIN = 0.0f
    const val BUBBLE_ALPHA_MAX = 1.0f
    // 14 intervals (15 positions) — same count as the size slider so the
    // two sliders feel similar. Range here is wider (1.0 vs 0.7), so the
    // step value is smaller (1/14 ≈ 0.0714 vs size's 0.05). Existing
    // off-grid stored values are snapped to the nearest position by
    // snapBubbleAlpha() on read, so users see at most a one-position
    // shift after upgrade.
    const val BUBBLE_ALPHA_STEP = 1f / 14f
    // Slight translucency by default — enough to blend with a patterned
    // wallpaper, not so much that the bird gets lost. Lands on the
    // grid: 12 × (1/14) = 0.857, closest grid stop to the original 0.85
    // so existing users get the same visual.
    const val BUBBLE_ALPHA_DEFAULT = 12f / 14f

    // Default ON since 1.0.4 — vibration is a "feature" we want users to
    // experience out of the box. The MainActivity shows a one-time FYI card
    // explaining this and offering a one-tap path to disable, so users
    // who don't want it can flip it off without hunting through settings.
    fun isHapticsEnabled(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_HAPTICS_ENABLED, true)

    fun setHapticsEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_HAPTICS_ENABLED, enabled)
            .apply()
    }

    /** Whether the bubble should hide itself when an active search field
     *  has focus (Chrome address bar, app search boxes, etc). Off by
     *  default — existing users mustn't suddenly find the bubble missing
     *  in places it used to appear. Surfaced as a Settings toggle. */
    fun isHideInSearchEnabled(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_HIDE_IN_SEARCH, false)

    fun setHideInSearchEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_HIDE_IN_SEARCH, enabled)
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

    // Bubble size scale relative to the baseline 56dp disc. 1.0 = stock,
    // 0.7 = compact (≈39 dp), 1.4 = chunky (≈78 dp). Same slider-step
    // discipline as alpha — Material Slider throws on off-step values.
    const val BUBBLE_SIZE_MIN = 0.7f
    const val BUBBLE_SIZE_MAX = 1.4f
    const val BUBBLE_SIZE_STEP = 0.05f
    const val BUBBLE_SIZE_DEFAULT = 1.0f

    fun getBubbleSize(context: Context): Float {
        val raw = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getFloat(KEY_BUBBLE_SIZE, BUBBLE_SIZE_DEFAULT)
        return snapBubbleSize(raw)
    }

    fun setBubbleSize(context: Context, size: Float) {
        val snapped = snapBubbleSize(size)
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putFloat(KEY_BUBBLE_SIZE, snapped)
            .apply()
    }

    private fun snapBubbleSize(value: Float): Float {
        val clamped = value.coerceIn(BUBBLE_SIZE_MIN, BUBBLE_SIZE_MAX)
        val steps = Math.round((clamped - BUBBLE_SIZE_MIN) / BUBBLE_SIZE_STEP)
        // No extra 2-decimal rounding here — same reasoning as
        // snapBubbleAlpha. Step 0.05 happens to be 2-decimal-clean today
        // so the previous version didn't crash, but the rounding would
        // misalign any future step that isn't (e.g. 1/14 = 0.0714...).
        val snapped = BUBBLE_SIZE_MIN + steps * BUBBLE_SIZE_STEP
        return snapped.coerceIn(BUBBLE_SIZE_MIN, BUBBLE_SIZE_MAX)
    }

    // WindowManager Y offset of the floating bubble — set after the user
    // drags it. Default 0 = vertical centre (the LayoutParams gravity is
    // CENTER_VERTICAL, so Y is interpreted as offset from centre). Stored
    // so the bubble doesn't snap back to the middle every time the system
    // restarts the accessibility service.
    fun getBubbleY(context: Context): Int =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getInt(KEY_BUBBLE_Y, 0)

    fun setBubbleY(context: Context, y: Int) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putInt(KEY_BUBBLE_Y, y)
            .apply()
    }

    // Which screen edge the bubble snaps to. Default right (matches the
    // accessibility-overlay convention on most Android voice/translate apps).
    fun getBubbleSide(context: Context): String =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getString(KEY_BUBBLE_SIDE, BUBBLE_SIDE_RIGHT) ?: BUBBLE_SIDE_RIGHT

    fun setBubbleSide(context: Context, side: String) {
        require(side == BUBBLE_SIDE_LEFT || side == BUBBLE_SIDE_RIGHT) {
            "Invalid bubble side: $side"
        }
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_BUBBLE_SIDE, side)
            .apply()
    }

    // One-time "What's new in 1.0.4" card on the main screen. Once
    // dismissed, never shown again. New users (post-onboarding) and
    // upgrading users (skip onboarding) both see it once.
    fun isWhatsNewHintDismissed(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_WHATS_NEW_HINT_DISMISSED, false)

    fun setWhatsNewHintDismissed(context: Context, dismissed: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_WHATS_NEW_HINT_DISMISSED, dismissed)
            .apply()
    }

    // User dictionary — raw text in "key=value" per line format. Parsing,
    // word-boundary regex compilation and replacement happen in
    // [com.govorun.lite.transcriber.Dictionary]. Stored as a single string
    // so import/export trivially round-trips through ACTION_OPEN/CREATE_DOCUMENT.
    fun getDictionary(context: Context): String =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getString(KEY_DICTIONARY, "") ?: ""

    fun setDictionary(context: Context, text: String) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_DICTIONARY, text)
            .apply()
    }

    // Master on/off for the dictionary. Default OFF so a brand-new user
    // can browse the editor (with starter examples) without their next
    // dictation being silently transformed by rules they never asked for.
    // Enabling it is a deliberate one-tap action inside the dictionary
    // screen, after the user has seen what's there.
    fun isDictionaryEnabled(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_DICTIONARY_ENABLED, false)

    fun setDictionaryEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_DICTIONARY_ENABLED, enabled)
            .apply()
    }

    fun getPauseLength(context: Context): String {
        val raw = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getString(KEY_PAUSE_LENGTH, PAUSE_DEFAULT) ?: PAUSE_DEFAULT
        return when (raw) {
            PAUSE_SHORT, PAUSE_MEDIUM, PAUSE_LONG -> raw
            else -> PAUSE_DEFAULT
        }
    }

    fun setPauseLength(context: Context, value: String) {
        require(value == PAUSE_SHORT || value == PAUSE_MEDIUM || value == PAUSE_LONG) {
            "Invalid pause length: $value"
        }
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_PAUSE_LENGTH, value)
            .apply()
    }

    fun getPauseLengthSeconds(context: Context): Float = when (getPauseLength(context)) {
        PAUSE_MEDIUM -> PAUSE_MEDIUM_SECONDS
        PAUSE_LONG -> PAUSE_LONG_SECONDS
        else -> PAUSE_SHORT_SECONDS
    }

    /**
     * One-time migration when a user first launches 1.0.8. Bumps Short →
     * Medium because 0.5s was too aggressive for many users (multiple
     * complaints about sentences getting cut mid-thought). We treat Short
     * as "kept the default" since it WAS the default in 1.0.7 — anyone
     * who wanted faster reaction will switch back, and they get a clear
     * note in "What's new" telling them how. Long is not touched — that's
     * an explicit choice and we honour it.
     *
     * Idempotent via [KEY_V108_PAUSE_MIGRATED] — runs once per install
     * lifetime, even across cold starts and process restarts.
     */
    fun migrateTo108PauseDefault(context: Context) {
        val prefs = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
        if (prefs.getBoolean(KEY_V108_PAUSE_MIGRATED, false)) return
        if (getPauseLength(context) == PAUSE_SHORT) {
            setPauseLength(context, PAUSE_MEDIUM)
        }
        prefs.edit().putBoolean(KEY_V108_PAUSE_MIGRATED, true).apply()
    }

    // Always land on a multiple of the slider step — M3 Slider throws if
    // setValue() is called with anything between two steps. Compute via
    // step-index then multiply back: the result has the same float
    // representation that the Slider's own validateValues path computes
    // from stepSize, so they match. (An earlier version rounded to two
    // decimal places to "neutralise float drift", but that produced
    // 0.14 from a real grid stop of 0.142857 — Slider rejected it,
    // crashing SettingsActivity.onCreate.)
    private fun snapBubbleAlpha(value: Float): Float {
        val clamped = value.coerceIn(BUBBLE_ALPHA_MIN, BUBBLE_ALPHA_MAX)
        val steps = Math.round((clamped - BUBBLE_ALPHA_MIN) / BUBBLE_ALPHA_STEP)
        val snapped = BUBBLE_ALPHA_MIN + steps * BUBBLE_ALPHA_STEP
        return snapped.coerceIn(BUBBLE_ALPHA_MIN, BUBBLE_ALPHA_MAX)
    }
}
