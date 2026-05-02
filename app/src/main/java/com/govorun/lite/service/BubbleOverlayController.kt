package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.annotation.SuppressLint
import android.content.Context
import android.graphics.PixelFormat
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import androidx.appcompat.view.ContextThemeWrapper
import com.google.android.material.color.DynamicColors
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.Prefs

/**
 * Owns the floating bubble: WindowManager params, view lifecycle, touch
 * handling (tap / hold / drag), Settings-driven re-layout, and the
 * recording-indicator state. Extracted from LiteAccessibilityService to
 * keep that file under the project's 500-line target and to give the
 * bubble's gesture handling a single self-contained home.
 *
 * The controller owns nothing about *when* the bubble should be visible
 * (that's the service's IME-state logic) or *what to do* on tap/hold
 * (that's the service's recording lifecycle). It exposes:
 *   - lifecycle calls: [create] / [destroy] / [rebuild]
 *   - state setters: [setRecording] / [setVisibility]
 *   - prefs reactions: [applyAlpha] / [applySize] / [applySide]
 *   - reads: [isVisible] / [getView]
 *
 * and pulls the recording-control + state queries back through the
 * [Callbacks] interface.
 */
class BubbleOverlayController(
    private val service: AccessibilityService,
    private val callbacks: Callbacks
) {

    interface Callbacks {
        /** Quick tap (under [HOLD_DELAY_MS]). Service decides start vs stop. */
        fun onTap()
        /** Long press detected; service starts hold-to-talk recording. */
        fun onHoldStart()
        /** Hold released. silent=true on CANCEL/system abort, false on natural release. */
        fun onHoldStop(silent: Boolean)
        /** Service's current recording state, used to interpret tap as toggle. */
        fun isVadActive(): Boolean
    }

    companion object {
        private const val TAG = "BubbleOverlay"
        private const val DRAG_THRESHOLD_DP = 10f
        // Press duration after which a touch is treated as "hold-to-talk"
        // instead of a tap-toggle. 250 ms is comfortably longer than a
        // normal tap (~50–100 ms) and shorter than a deliberate long-press.
        private const val HOLD_DELAY_MS = 250L
        // Smaller-than-drag slop used to detect *intentional* finger
        // movement during the hold-wait window — without it, slow back-edge
        // swipes (which haven't crossed the 10dp drag threshold yet by the
        // time HOLD_DELAY_MS elapses) trigger a brief recording flash.
        private const val HOLD_MOVEMENT_SLOP_DP = 5f
    }

    private val windowManager =
        service.getSystemService(Context.WINDOW_SERVICE) as WindowManager
    private var bubbleView: BubbleView? = null
    private var bubbleParams: WindowManager.LayoutParams? = null

    /** Build the LayoutParams (once) and attach the bubble. */
    fun create(initiallyVisible: Boolean) {
        if (bubbleParams == null) {
            bubbleParams = WindowManager.LayoutParams(
                WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                // HARDWARE_ACCELERATED added: TYPE_ACCESSIBILITY_OVERLAY by
                // default may render through the software path, which on
                // Android 16 + Pixel 10 + Gboard creates visible drag lag
                // when the bubble crosses the IME surface (the compositor
                // has to recomposite the IME under the overlay every move).
                // GPU rendering avoids that hot path. On older OS / hardware
                // this is a no-op or a small win.
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                    WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED,
                PixelFormat.TRANSLUCENT
            ).apply {
                gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
                x = 16
                // Restore the user's last drag position. Without this, every
                // time Android restarts our service (memory pressure, OEM
                // battery-saver killing the process, accessibility unbind/rebind
                // cycle) the bubble snaps back to vertical centre.
                y = Prefs.getBubbleY(service)
            }
        }
        attachFreshBubble(initiallyVisible)
    }

    /** Tear down the overlay; called from service onDestroy. */
    fun destroy() {
        bubbleView?.let { v ->
            try { windowManager.removeView(v) } catch (_: Exception) {}
        }
        bubbleView = null
    }

    /** Rebuild the bubble view (preserves params + position). Used on
     *  Settings changes that affect view sizing, and on configuration
     *  changes (wallpaper colour) where DynamicColors needs re-wrapping. */
    fun rebuild(initiallyVisible: Boolean) {
        attachFreshBubble(initiallyVisible)
    }

    private var recordingActive = false
    // Transient screen-keep-on override driven by VAD speech detection —
    // the service flips this true on speech-start and false after a short
    // grace following speech-end, so active dictation isn't killed by the
    // system's screen timeout even when the user opted out of the
    // persistent keep-screen-on switch.
    private var speechActiveOverride = false

    /** Toggle the recording indicator (red bubble). The screen-keep-on
     *  flag is held when the user has opted into it via Settings
     *  (Prefs.isKeepScreenOnEnabled), OR transiently while VAD reports
     *  active speech (see setKeepScreenOnOverride). Without the flag,
     *  the screen sleeps per the user's system timeout and the
     *  service's screenOffReceiver picks up ACTION_SCREEN_OFF and stops
     *  the session automatically. */
    fun setRecording(active: Boolean) {
        bubbleView?.setRecording(active)
        recordingActive = active
        if (!active) speechActiveOverride = false
        applyKeepScreenFlag()
    }

    /** Called by the service when VAD edge events arrive — true while
     *  speech is currently audible, false after the grace window expires.
     *  No effect outside an active recording. */
    fun setKeepScreenOnOverride(active: Boolean) {
        speechActiveOverride = active
        applyKeepScreenFlag()
    }

    private fun applyKeepScreenFlag() {
        val params = bubbleParams ?: return
        val keepScreenOn = recordingActive &&
            (Prefs.isKeepScreenOnEnabled(service) || speechActiveOverride)
        params.flags = if (keepScreenOn) {
            params.flags or WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
        } else {
            params.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON.inv()
        }
        try { windowManager.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
    }

    /** Push the visibility decision into the view on its own looper. */
    fun setVisibility(visibility: Int) {
        bubbleView?.post { bubbleView?.visibility = visibility }
    }

    fun isVisible(): Boolean = bubbleView?.visibility == View.VISIBLE

    fun getView(): View? = bubbleView

    /** Re-read alpha pref into the existing view; no rebuild needed. */
    fun applyAlpha() {
        bubbleView?.setIdleAlpha(Prefs.getBubbleAlpha(service))
    }

    /** Size affects onMeasure, so rebuild the bubble (cheaper than
     *  trying to in-place re-layout an overlay with running animators). */
    fun applySize(initiallyVisible: Boolean) {
        attachFreshBubble(initiallyVisible)
    }

    /** Side change is just gravity — update LayoutParams in place. */
    fun applySide() {
        val params = bubbleParams ?: return
        params.gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
        try { windowManager.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
    }

    private fun bubbleHorizontalGravity(): Int =
        if (Prefs.getBubbleSide(service) == Prefs.BUBBLE_SIDE_LEFT) Gravity.START else Gravity.END

    /**
     * Services don't get DynamicColors applied automatically (that helper
     * is activity-scoped). Wrapping explicitly here is what makes the
     * bubble pick up the user's wallpaper accent.
     */
    private fun bubbleContext(): Context {
        val themed = ContextThemeWrapper(service, R.style.Theme_GovorunLite)
        return DynamicColors.wrapContextIfAvailable(themed)
    }

    @SuppressLint("ClickableViewAccessibility")
    private fun attachFreshBubble(initiallyVisible: Boolean) {
        val params = bubbleParams ?: return
        bubbleView?.let {
            try { windowManager.removeView(it) } catch (_: Exception) {}
        }
        val fresh = BubbleView(bubbleContext()).apply {
            setIdleAlpha(Prefs.getBubbleAlpha(service))
            visibility = if (initiallyVisible) View.VISIBLE else View.GONE
            // Note: previously this view was wrapped in a hardware layer
            // via setLayerType(LAYER_TYPE_HARDWARE) on the assumption it
            // would help drag perf. The actual drag lag we were chasing
            // turned out to be SurfaceFlinger composition, not view
            // rendering — and a hardware layer is counterproductive for
            // a view that animates its own contents (halo pulse,
            // recording pulse) because every invalidate() re-uploads
            // the cached texture. The window-level FLAG_HARDWARE_ACCELERATED
            // already guarantees GPU rendering for the surface; this
            // extra layer was redundant work.
        }
        installTouchListener(fresh, params)
        try { windowManager.addView(fresh, params) } catch (e: Exception) {
            Log.e(TAG, "Failed to add bubble view", e)
        }
        bubbleView = fresh
    }

    @SuppressLint("ClickableViewAccessibility")
    private fun installTouchListener(view: BubbleView, params: WindowManager.LayoutParams) {
        val dragThresholdPx = DRAG_THRESHOLD_DP * service.resources.displayMetrics.density
        val holdMovementSlopPx = HOLD_MOVEMENT_SLOP_DP * service.resources.displayMetrics.density
        view.setOnTouchListener(object : View.OnTouchListener {
            private var initialY = 0
            private var initialTouchY = 0f
            private var initialTouchX = 0f
            private var lastTouchX = 0f
            private var lastTouchY = 0f
            private var dragged = false
            private var holdStarted = false
            private val holdHandler = Handler(Looper.getMainLooper())
            // Posted on DOWN, fires after HOLD_DELAY_MS. Only promotes the
            // gesture to hold-to-talk if the finger is genuinely still —
            // the "still" check uses HOLD_MOVEMENT_SLOP_DP (5dp), tighter
            // than the 10dp drag threshold, so a slow back-edge swipe that
            // hasn't yet tripped drag mode doesn't accidentally start
            // recording with a brief red flash before being cancelled.
            private val holdRunnable = Runnable {
                if (dragged || holdStarted) return@Runnable
                val movedX = Math.abs(lastTouchX - initialTouchX)
                val movedY = Math.abs(lastTouchY - initialTouchY)
                if (movedX > holdMovementSlopPx || movedY > holdMovementSlopPx) {
                    // Finger is in slow motion — treat as the start of a
                    // gesture, not a hold. Don't start recording.
                    return@Runnable
                }
                holdStarted = true
                callbacks.onHoldStart()
            }
            override fun onTouch(v: View, event: MotionEvent): Boolean {
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        initialY = params.y
                        initialTouchY = event.rawY
                        initialTouchX = event.rawX
                        lastTouchX = event.rawX
                        lastTouchY = event.rawY
                        dragged = false
                        holdStarted = false
                        holdHandler.postDelayed(holdRunnable, HOLD_DELAY_MS)
                        return true
                    }
                    MotionEvent.ACTION_MOVE -> {
                        lastTouchX = event.rawX
                        lastTouchY = event.rawY
                        // Once hold-recording started, the bubble is locked
                        // in place. Any finger movement is ignored — release
                        // (UP) is the only way to stop. Without this, the
                        // bubble would drag along with the user's tiny finger
                        // adjustments while they're talking.
                        if (holdStarted) return true
                        val dy = event.rawY - initialTouchY
                        val dx = event.rawX - initialTouchX
                        // ANY-direction threshold check: a back-edge swipe
                        // that crosses the bubble is mostly horizontal —
                        // before this fix it never tripped the (vertical-only)
                        // drag flag, so on UP it looked like a tap and
                        // started/stopped recording.
                        if (Math.abs(dy) > dragThresholdPx || Math.abs(dx) > dragThresholdPx) {
                            if (!dragged) {
                                dragged = true
                                holdHandler.removeCallbacks(holdRunnable)
                            }
                            // Bubble itself only moves vertically — horizontal
                            // delta just blocks tap activation, doesn't drag.
                            if (Math.abs(dy) > dragThresholdPx) {
                                params.y = initialY + dy.toInt()
                                try {
                                    windowManager.updateViewLayout(bubbleView, params)
                                } catch (_: Exception) {}
                            }
                        }
                        return true
                    }
                    MotionEvent.ACTION_UP -> {
                        holdHandler.removeCallbacks(holdRunnable)
                        if (dragged) {
                            // Persist final Y so the bubble lands in the same
                            // place after the next service restart.
                            Prefs.setBubbleY(service, params.y)
                            return true
                        }
                        if (holdStarted) {
                            // Hold-to-talk: releasing the finger stops the
                            // recording (and triggers VAD pipeline → paste).
                            callbacks.onHoldStop(silent = false)
                            return true
                        }
                        callbacks.onTap()
                        return true
                    }
                    MotionEvent.ACTION_CANCEL -> {
                        // CANCEL means the gesture was aborted by the system
                        // — usually because the finger left the view bounds
                        // mid-touch (back-edge swipe across the bubble is
                        // the typical cause). Do NOT treat this as a tap or
                        // hold-release; just clean up.
                        holdHandler.removeCallbacks(holdRunnable)
                        if (holdStarted) callbacks.onHoldStop(silent = true)
                        if (dragged) Prefs.setBubbleY(service, params.y)
                        return true
                    }
                }
                return false
            }
        })
    }
}
