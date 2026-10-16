package com.govorun.lite.util

import android.app.Activity
import android.content.Context
import android.media.AudioAttributes
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import com.govorun.lite.service.LiteAccessibilityService

/**
 * Single source of truth for all haptic feedback in the app.
 *
 * Strategy: prefer [View.performHapticFeedback] with FLAG_IGNORE_GLOBAL_SETTING
 * over the low-level Vibrator API. Reasons:
 *
 *  • Immune to the system "Touch feedback" toggle, DnD, Battery Saver —
 *    user can't accidentally mute our haptics by toggling unrelated stuff.
 *  • Routes through the system haptic-feedback pipeline → device-native
 *    feel (sharp click on Pixel, soft tap on Samsung, etc.) instead of the
 *    flat "bzzz" of a raw waveform.
 *  • One mechanism across the whole project — no half-and-half where
 *    bubble haptics fire but settings haptics don't.
 *
 * View source preference:
 *  1. The floating bubble in our running AccessibilityService (works from
 *     anywhere the user can see the bubble — including background callers).
 *  2. Activity decor view (when called from an Activity context).
 *  3. Last-resort: low-level Vibrator with USAGE_TOUCH attributes.
 *
 * Semantic methods (caller picks intent, not waveform):
 *  • [tap]       — single soft confirm
 *  • [doubleTap] — two confirms (important event done — record sent)
 *  • [longPress] — firm tap (something deliberate just started — record on)
 *  • [reject]    — error/cancel
 *
 * No-op (silently returns) when haptics are disabled in [Prefs] or there's
 * no vibrator hardware. VIBRATE permission is in the manifest.
 */
object Haptics {

    private const val TAG = "Haptics"
    private const val DOUBLE_TAP_GAP_MS = 120L

    fun tap(context: Context) = play(context, HapticFeedbackConstants.CONFIRM, repeats = 1)
    fun doubleTap(context: Context) = play(context, HapticFeedbackConstants.CONFIRM, repeats = 2)
    fun longPress(context: Context) = play(context, HapticFeedbackConstants.LONG_PRESS, repeats = 1)
    fun reject(context: Context) = play(context, HapticFeedbackConstants.REJECT, repeats = 2)

    private fun play(context: Context, type: Int, repeats: Int) {
        if (!Prefs.isHapticsEnabled(context)) return
        val view = findView(context)
        if (view != null) {
            performHaptic(view, type, repeats)
        } else {
            // Fallback path — user hasn't yet enabled the accessibility
            // service AND we're called from a non-Activity context (e.g.
            // a worker). Should be rare in practice.
            fallbackVibrate(context, repeats)
        }
    }

    /** Find the best View to dispatch haptic feedback through. */
    private fun findView(context: Context): View? {
        LiteAccessibilityService.getBubbleViewIfAttached()?.let { return it }
        if (context is Activity) return context.window.decorView
        return null
    }

    private fun performHaptic(view: View, type: Int, repeats: Int) {
        val flags = HapticFeedbackConstants.FLAG_IGNORE_GLOBAL_SETTING
        view.performHapticFeedback(type, flags)
        if (repeats > 1) {
            view.postDelayed({
                view.performHapticFeedback(type, flags)
            }, DOUBLE_TAP_GAP_MS)
        }
    }

    private fun fallbackVibrate(context: Context, repeats: Int) {
        val v = vibrator(context) ?: return
        if (!v.hasVibrator()) return
        val timings = if (repeats >= 2) longArrayOf(0L, 50L, 80L, 50L) else longArrayOf(0L, 50L)
        val effect = VibrationEffect.createWaveform(timings, -1)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val attrs = VibrationAttributes.Builder()
                .setUsage(VibrationAttributes.USAGE_TOUCH)
                .build()
            v.vibrate(effect, attrs)
        } else {
            @Suppress("DEPRECATION")
            val audioAttrs = AudioAttributes.Builder()
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                .build()
            v.vibrate(effect, audioAttrs)
        }
        Log.d(TAG, "Fallback vibrator path (no view available, repeats=$repeats)")
    }

    private fun vibrator(context: Context): Vibrator? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            (context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager)
                ?.defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
        }
}
