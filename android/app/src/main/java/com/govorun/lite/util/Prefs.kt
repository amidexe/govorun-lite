package com.govorun.lite.util

import android.content.Context

/**
 * Single place that knows the app's SharedPreferences file name + keys.
 * Keeps key spelling consistent across UI (where the user toggles it) and
 * the AccessibilityService (which reads it on every bubble tap).
 */
object Prefs {

    private const val FILE_NAME = StorageKeys.PREFS_FILE
    private const val KEY_HAPTICS_ENABLED = StorageKeys.Key.HAPTICS_ENABLED
    private const val KEY_HIDE_IN_SEARCH = StorageKeys.Key.HIDE_IN_SEARCH
    private const val KEY_BUBBLE_ALPHA = StorageKeys.Key.BUBBLE_ALPHA
    private const val KEY_BUBBLE_SIZE = StorageKeys.Key.BUBBLE_SIZE
    private const val KEY_BUBBLE_Y_PORTRAIT = StorageKeys.Key.BUBBLE_Y_PORTRAIT
    private const val KEY_BUBBLE_Y_LANDSCAPE = StorageKeys.Key.BUBBLE_Y_LANDSCAPE
    private const val KEY_BUBBLE_SIDE = StorageKeys.Key.BUBBLE_SIDE
    private const val KEY_DICTIONARY = StorageKeys.Key.DICTIONARY
    private const val KEY_DICTIONARY_ENABLED = StorageKeys.Key.DICTIONARY_ENABLED
    private const val KEY_PAUSE_LENGTH = StorageKeys.Key.PAUSE_LENGTH
    private const val KEY_KEEP_SCREEN_ON = StorageKeys.Key.KEEP_SCREEN_ON
    private const val KEY_AUTO_STOP_MINUTES = StorageKeys.Key.AUTO_STOP_MINUTES

    const val BUBBLE_SIDE_RIGHT = "right"
    const val BUBBLE_SIDE_LEFT = "left"

    // VAD silence threshold presets — how long the silence must last before
    // the VAD considers the utterance finished. MEDIUM is the shared default;
    // explicitly saved choices are preserved in every dictation mode.
    // LONG gives "thinking pauses" room before the model commits a paragraph
    // — this is what the RuStore feedback ("she pauses to think and the app
    // cuts the sentence") asked for.
    const val PAUSE_SHORT = "short"
    const val PAUSE_MEDIUM = "medium"
    const val PAUSE_LONG = "long"
    const val PAUSE_DEFAULT = PAUSE_MEDIUM

    // Seconds, fed verbatim into SileroVadModelConfig.minSilenceDuration.
    // Doubling per step keeps the difference perceptually obvious.
    const val PAUSE_SHORT_SECONDS = 0.5f
    const val PAUSE_MEDIUM_SECONDS = 1.0f
    const val PAUSE_LONG_SECONDS = 2.0f

    // Online text post-processing. It is deliberately off by default and
    // can only be enabled from the dedicated screen after a disclosure.
    // Keep the persisted style as a stable API id rather than a translated
    // label so changing UI copy cannot silently change behaviour.
    fun isStylizationEnabled(context: Context): Boolean =
        StylizationPrefs.isEnabled(context)

    fun setStylizationEnabled(context: Context, enabled: Boolean) {
        StylizationPrefs.setEnabled(context, enabled)
    }

    fun getStylizationStyle(context: Context): String =
        StylizationPrefs.style(context)

    fun setStylizationStyle(context: Context, styleId: String) {
        StylizationPrefs.setStyle(context, styleId)
    }

    fun getStylizationCustomInstruction(context: Context): String =
        StylizationPrefs.customInstruction(context)

    fun setStylizationCustomInstruction(context: Context, instruction: String) {
        StylizationPrefs.setCustomInstruction(context, instruction)
    }

    fun hasStylizationConsent(context: Context, currentVersion: Int): Boolean =
        StylizationPrefs.hasConsent(context, currentVersion)

    fun getStylizationConsentVersion(context: Context): Int =
        StylizationPrefs.consentVersion(context)

    fun dismissStylizationTermsUpdate(context: Context) {
        StylizationPrefs.dismissTermsUpdate(context)
    }

    fun acceptStylizationTerms(
        context: Context,
        version: Int,
        acceptedAtMillis: Long = System.currentTimeMillis(),
    ) {
        StylizationPrefs.acceptTerms(context, version, acceptedAtMillis)
    }

    fun getStylizationConsentAt(context: Context): Long =
        StylizationPrefs.consentAt(context)

    // Clamp range for every bubble background fill. Floor 0.0 = fully
    // transparent background, leaving only the opaque bird/actions/activity
    // dots visible. Ceiling 1.0 is fully opaque. Foreground glyph colours
    // remain theme-driven and are never made transparent by this setting.
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

    // WindowManager Y offset of the floating bubble, stored independently for
    // portrait and landscape. A single offset cannot describe both layouts:
    // the keyboard height and available vertical space change radically after
    // rotation. Default 0 = vertical centre (LayoutParams uses CENTER_VERTICAL).
    fun getBubbleY(context: Context): Int {
        val prefs = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
        return prefs.getInt(bubbleYKey(context.resources.configuration.orientation),
            prefs.getInt(StorageKeys.Key.BUBBLE_Y, 0))
    }

    fun setBubbleY(context: Context, y: Int) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putInt(bubbleYKey(context.resources.configuration.orientation), y)
            .apply()
    }

    fun resetBubbleY(context: Context) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .remove(KEY_BUBBLE_Y_PORTRAIT)
            .remove(KEY_BUBBLE_Y_LANDSCAPE)
            .remove(StorageKeys.Key.BUBBLE_Y)
            .apply()
    }

    private fun bubbleYKey(orientation: Int): String =
        if (orientation == android.content.res.Configuration.ORIENTATION_LANDSCAPE) {
            KEY_BUBBLE_Y_LANDSCAPE
        } else {
            KEY_BUBBLE_Y_PORTRAIT
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

    // "Keep screen on while listening" — opt-in override of the default
    // hands-off behaviour. OFF (default since 1.0.11): the bubble does not
    // hold FLAG_KEEP_SCREEN_ON, so the screen sleeps per the user's system
    // timeout exactly like any other app, and the existing screenOffReceiver
    // stops recording automatically. ON: hold the flag for the whole session,
    // matching the pre-1.0.11 behaviour (intended for users with phone on
    // charger, wall-mounted tablet, long-form dictation).
    fun isKeepScreenOnEnabled(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_KEEP_SCREEN_ON, false)

    fun setKeepScreenOnEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_KEEP_SCREEN_ON, enabled)
            .apply()
    }

    // Wall-clock cap on a single recording session. Only relevant when
    // Keep-Screen-On is ON (otherwise the system screen timeout already
    // ends the session via screenOffReceiver). Stored as minutes; 0 means
    // "no cap". Surfaced as a 4-button toggle in Settings only when the
    // Keep-Screen-On switch above it is enabled.
    const val AUTO_STOP_15M = 15
    const val AUTO_STOP_1H = 60
    const val AUTO_STOP_3H = 180
    const val AUTO_STOP_OFF = 0
    const val AUTO_STOP_DEFAULT = AUTO_STOP_1H

    fun getAutoStopMinutes(context: Context): Int {
        val raw = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getInt(KEY_AUTO_STOP_MINUTES, AUTO_STOP_DEFAULT)
        return when (raw) {
            AUTO_STOP_15M, AUTO_STOP_1H, AUTO_STOP_3H, AUTO_STOP_OFF -> raw
            else -> AUTO_STOP_DEFAULT
        }
    }

    fun setAutoStopMinutes(context: Context, minutes: Int) {
        require(minutes == AUTO_STOP_15M || minutes == AUTO_STOP_1H ||
                minutes == AUTO_STOP_3H || minutes == AUTO_STOP_OFF) {
            "Invalid auto-stop minutes: $minutes"
        }
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putInt(KEY_AUTO_STOP_MINUTES, minutes)
            .apply()
    }

    // Edge margin — additional inset between the bubble disc and the screen edge,
    // stored in dp. 0 = as close to the edge as the halo ring allows (~17dp for
    // the disc itself at default size). Stored in dp so it scales correctly across
    // densities; converted to px in BubbleOverlayController.
    private const val KEY_BUBBLE_EDGE_MARGIN = StorageKeys.Key.BUBBLE_EDGE_MARGIN

    // Negative values push the view past the screen edge (halo clips, disc stays
    // visible). Overlay uses FLAG_LAYOUT_NO_LIMITS so negative x is honoured.
    // -16 dp ≈ disc right at the screen edge; 0 dp ≈ 17dp gap (just halo space);
    // +24 dp ≈ disc clearly inset. Step 4dp gives 11 positions total.
    const val BUBBLE_EDGE_MARGIN_MIN = -12
    const val BUBBLE_EDGE_MARGIN_MAX = 24
    const val BUBBLE_EDGE_MARGIN_STEP = 4
    const val BUBBLE_EDGE_MARGIN_DEFAULT = 0

    fun getBubbleEdgeMargin(context: Context): Int {
        val raw = context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getInt(KEY_BUBBLE_EDGE_MARGIN, BUBBLE_EDGE_MARGIN_DEFAULT)
        return snapBubbleEdgeMargin(raw)
    }

    fun setBubbleEdgeMargin(context: Context, dp: Int) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putInt(KEY_BUBBLE_EDGE_MARGIN, snapBubbleEdgeMargin(dp))
            .apply()
    }

    private fun snapBubbleEdgeMargin(value: Int): Int {
        val clamped = value.coerceIn(BUBBLE_EDGE_MARGIN_MIN, BUBBLE_EDGE_MARGIN_MAX)
        val steps = Math.round((clamped - BUBBLE_EDGE_MARGIN_MIN).toFloat() / BUBBLE_EDGE_MARGIN_STEP)
        return (BUBBLE_EDGE_MARGIN_MIN + steps * BUBBLE_EDGE_MARGIN_STEP)
            .coerceIn(BUBBLE_EDGE_MARGIN_MIN, BUBBLE_EDGE_MARGIN_MAX)
    }

    // Lock bubble position — when ON, drag gestures on the bubble are ignored.
    // OFF by default so existing users are unaffected.
    private const val KEY_BUBBLE_POSITION_LOCKED = StorageKeys.Key.BUBBLE_POSITION_LOCKED

    fun isBubblePositionLocked(context: Context): Boolean =
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_BUBBLE_POSITION_LOCKED, false)

    fun setBubblePositionLocked(context: Context, locked: Boolean) {
        context.getSharedPreferences(FILE_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_BUBBLE_POSITION_LOCKED, locked)
            .apply()
    }

    const val APP_FILTER_BLACKLIST = "blacklist"
    const val APP_FILTER_WHITELIST = "whitelist"

    fun getAppFilterMode(context: Context): String =
        AppFilterPrefs.mode(context)

    fun setAppFilterMode(context: Context, mode: String) {
        AppFilterPrefs.setMode(context, mode)
    }

    fun getAppFilterPackages(context: Context): Set<String> =
        AppFilterPrefs.packages(context)

    fun setAppFilterPackages(context: Context, pkgs: Set<String>) {
        AppFilterPrefs.setPackages(context, pkgs)
    }

    /** Returns true when the bubble should be HIDDEN in this app package. */
    fun isAppFiltered(context: Context, pkg: String?): Boolean {
        pkg ?: return false
        return AppFilterPrefs.isFiltered(context, pkg)
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
