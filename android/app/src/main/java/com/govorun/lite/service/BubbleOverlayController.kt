package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.annotation.SuppressLint
import android.content.Context
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.os.SystemClock
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import androidx.appcompat.view.ContextThemeWrapper
import com.google.android.material.color.DynamicColors
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.overlay.BubblePlacement
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.UiAppearancePrefs
import com.govorun.lite.model.settings.UiThemeMode

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

    enum class RecordingUiMode {
        /** Existing pause-split offline dictation. */
        LEGACY,
        /** Press, keep holding, release to submit the complete phrase. */
        HOLD,
        /** Tap once, speak freely, then explicitly confirm or cancel. */
        CONFIRM,
    }

    interface Callbacks {
        fun onPointerDown() {}
        fun onPointerCancel() {}
        /** Quick tap. Service decides start vs stop. */
        fun onTap()
        /** Long press detected; service starts hold-to-talk recording. */
        fun onHoldStart()
        /** Hold released. silent=true on CANCEL/system abort, false on natural release. */
        fun onHoldStop(silent: Boolean)
        /** Anchored bird action in tap-to-confirm mode. */
        fun onConfirm()
        /** Explicit cross in tap-to-confirm mode. */
        fun onCancel()
        /** Service's current recording state, used to interpret tap as toggle. */
        fun isVadActive(): Boolean
    }

    companion object {
        private const val TAG = "BubbleOverlay"
    }

    private val windowManager =
        service.getSystemService(Context.WINDOW_SERVICE) as WindowManager
    private val screenAwakeController = RecordingScreenAwakeController(service)
    private var bubbleView: BubbleView? = null
    private var touchView: BubbleInputView? = null
    private var bubbleParams: WindowManager.LayoutParams? = null
    private var touchParams: WindowManager.LayoutParams? = null

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
                    WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                    WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED or
                    // Allows negative x so the halo can clip at the screen edge
                    // when the user sets a small edge margin.
                    WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                PixelFormat.TRANSLUCENT
            ).apply {
                gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
                x = edgeMarginPx()
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
        callbacks.onPointerCancel()
        screenAwakeController.release()
        touchView?.let { v ->
            try { windowManager.removeView(v) } catch (_: Exception) {}
        }
        bubbleView?.let { v ->
            try { windowManager.removeView(v) } catch (_: Exception) {}
        }
        touchView = null
        bubbleView = null
    }

    /** Rebuild the bubble view (preserves params + position). Used on
     *  Settings changes that affect view sizing, and on configuration
     *  changes (wallpaper colour) where DynamicColors needs re-wrapping. */
    fun rebuild(initiallyVisible: Boolean) {
        // Configuration may have switched portrait ↔ landscape. Each layout
        // has its own user-chosen Y offset; load it before attaching the new
        // overlay view instead of carrying the previous orientation's value.
        bubbleParams?.y = Prefs.getBubbleY(service)
        attachFreshBubble(initiallyVisible)
    }

    private var recordingActive = false
    private var recordingUiMode = RecordingUiMode.LEGACY
    // Transient screen-keep-on override driven by VAD speech detection —
    // the service flips this true on speech-start and false after a short
    // grace following speech-end, so active dictation isn't killed by the
    // system's screen timeout even when the user opted out of the
    // persistent keep-screen-on switch.
    private var speechActiveOverride = false
    // True while the model is processing audio after hold-to-talk release.
    // Keeps the screen alive unconditionally until the result lands.
    private var processingActive = false
    private var lastFallbackSignalAt = 0L

    /** Toggle the recording indicator (red bubble). The screen-keep-on
     *  flag is held when the user has opted into it via Settings
     *  (Prefs.isKeepScreenOnEnabled), OR transiently while VAD reports
     *  active speech (see setKeepScreenOnOverride). Without the flag,
     *  the screen sleeps per the user's system timeout and the
     *  service's screenOffReceiver picks up ACTION_SCREEN_OFF and stops
     *  the session automatically. */
    fun setRecording(
        active: Boolean,
        uiMode: RecordingUiMode = recordingUiMode,
    ) {
        recordingUiMode = uiMode
        bubbleView?.setPresentation(
            when {
                !active -> BubbleView.Presentation.IDLE
                uiMode == RecordingUiMode.HOLD -> BubbleView.Presentation.HOLD_RECORDING
                uiMode == RecordingUiMode.CONFIRM -> BubbleView.Presentation.CONFIRM_RECORDING
                else -> BubbleView.Presentation.LEGACY_RECORDING
            },
            anchorOnLeft = isAnchoredLeft(),
        )
        syncTouchWindow()
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

    /** Show the yellow "thinking" indicator while the model transcribes a
     *  hold-to-talk recording. Keeps the screen on unconditionally until
     *  [active] is set back to false in the onDone callback. */
    fun setProcessing(active: Boolean) {
        processingActive = active
        bubbleView?.setPresentation(
            if (active) BubbleView.Presentation.PROCESSING else BubbleView.Presentation.IDLE,
            anchorOnLeft = isAnchoredLeft(),
        )
        syncTouchWindow()
        applyKeepScreenFlag()
    }
    fun finishRecordingAndStartProcessing() {
        recordingActive = false
        speechActiveOverride = false
        processingActive = true
        bubbleView?.setPresentation(BubbleView.Presentation.PROCESSING, isAnchoredLeft())
        syncTouchWindow()
        applyKeepScreenFlag()
    }

    fun showStylizationFallback() {
        val now = SystemClock.elapsedRealtime()
        if (now - lastFallbackSignalAt < 2_000L) return
        lastFallbackSignalAt = now
        bubbleView?.post {
            bubbleView?.showStylizationFallback()
            syncTouchWindow()
        }
    }

    private fun applyKeepScreenFlag() {
        val params = bubbleParams ?: return
        // Screen stays on while recording (subject to prefs/VAD override)
        // OR unconditionally while the model is processing a hold-to-talk result.
        val keepScreenOn = processingActive ||
            (recordingActive && (Prefs.isKeepScreenOnEnabled(service) || speechActiveOverride))
        screenAwakeController.setActive(keepScreenOn)
        params.flags = if (keepScreenOn) {
            params.flags or WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
        } else {
            params.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON.inv()
        }
        try { windowManager.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
    }

    /** Push the visibility decision into the view on its own looper. */
    fun setVisibility(visibility: Int) {
        if (visibility != View.VISIBLE) callbacks.onPointerCancel()
        bubbleView?.post {
            bubbleView?.visibility = visibility
            touchView?.visibility = visibility
        }
    }

    private fun syncTouchWindow() {
        val control = bubbleView ?: return
        val input = touchView ?: return
        val params = touchParams ?: return
        val width = control.inputWidthPx()
        val height = control.controlHeightPx()
        val gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
        val x = edgeMarginPx()
        val y = bubbleParams?.y ?: params.y
        val layoutChanged = params.width != width ||
            params.height != height ||
            params.gravity != gravity ||
            params.x != x ||
            params.y != y
        params.width = width
        params.height = height
        params.gravity = gravity
        params.x = x
        params.y = y
        input.updateState(
            BubbleInputState(
                presentation = control.currentPresentation(),
                anchoredLeft = isAnchoredLeft(),
                width = width,
                height = height,
            ),
        )
        if (layoutChanged && input.isAttachedToWindow) {
            try { windowManager.updateViewLayout(input, params) } catch (_: Exception) {}
        }
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
        bubbleView?.setAnchoredLeft(isAnchoredLeft())
        touchParams?.gravity = params.gravity
        try { windowManager.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
        syncTouchWindow()
    }

    /** Edge margin change — update x in LayoutParams in place; no rebuild. */
    fun applyEdgeMargin() {
        val params = bubbleParams ?: return
        params.x = edgeMarginPx()
        touchParams?.x = params.x
        try { windowManager.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
        touchParams?.let { touch ->
            try { windowManager.updateViewLayout(touchView, touch) } catch (_: Exception) {}
        }
    }

    private fun edgeMarginPx(): Int =
        BubblePlacement.horizontalOffsetPx(service)

    private fun bubbleHorizontalGravity(): Int =
        if (Prefs.getBubbleSide(service) == Prefs.BUBBLE_SIDE_LEFT) Gravity.START else Gravity.END

    private fun isAnchoredLeft(): Boolean =
        Prefs.getBubbleSide(service) == Prefs.BUBBLE_SIDE_LEFT

    /**
     * Services don't get DynamicColors applied automatically (that helper
     * is activity-scoped). Wrapping explicitly here is what makes the
     * bubble pick up the user's wallpaper accent.
     */
    private fun bubbleContext(): Context {
        val appearance = UiAppearancePrefs.read(service)
        val themed = ContextThemeWrapper(service, R.style.Theme_GovorunLite)
        val nightMode = when (appearance.themeMode) {
            UiThemeMode.LIGHT -> Configuration.UI_MODE_NIGHT_NO
            UiThemeMode.DARK -> Configuration.UI_MODE_NIGHT_YES
            UiThemeMode.SYSTEM -> null
        }
        nightMode?.let { mode ->
            val override = Configuration(service.resources.configuration)
            override.uiMode = (override.uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or mode
            themed.applyOverrideConfiguration(override)
        }
        return if (appearance.dynamicColor) {
            DynamicColors.wrapContextIfAvailable(themed)
        } else {
            themed
        }
    }

    @SuppressLint("ClickableViewAccessibility")
    private fun attachFreshBubble(initiallyVisible: Boolean) {
        callbacks.onPointerCancel()
        val params = bubbleParams ?: return
        params.x = edgeMarginPx()
        touchView?.let {
            try { windowManager.removeView(it) } catch (_: Exception) {}
        }
        bubbleView?.let {
            try { windowManager.removeView(it) } catch (_: Exception) {}
        }
        val fresh = BubbleView(bubbleContext()).apply {
            setIdleAlpha(Prefs.getBubbleAlpha(service))
            visibility = if (initiallyVisible) View.VISIBLE else View.GONE
            setPresentation(
                when {
                    processingActive -> BubbleView.Presentation.PROCESSING
                    recordingActive && recordingUiMode == RecordingUiMode.HOLD ->
                        BubbleView.Presentation.HOLD_RECORDING
                    recordingActive && recordingUiMode == RecordingUiMode.CONFIRM ->
                        BubbleView.Presentation.CONFIRM_RECORDING
                    recordingActive -> BubbleView.Presentation.LEGACY_RECORDING
                    else -> BubbleView.Presentation.IDLE
                },
                anchorOnLeft = isAnchoredLeft(),
            )
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
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
        val inputParams = WindowManager.LayoutParams(
            fresh.inputWidthPx(),
            fresh.controlHeightPx(),
            WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT,
        ).apply {
            gravity = params.gravity
            x = params.x
            y = params.y
        }
        val input = BubbleInputView(bubbleContext(), callbacks).apply {
            visibility = if (initiallyVisible) View.VISIBLE else View.GONE
            updateState(
                BubbleInputState(
                    presentation = fresh.currentPresentation(),
                    anchoredLeft = isAnchoredLeft(),
                    width = fresh.inputWidthPx(),
                    height = fresh.controlHeightPx(),
                ),
            )
        }
        fresh.viewTreeObserver.addOnPreDrawListener {
            syncTouchWindow()
            true
        }
        BubbleGestureHandler(service, windowManager, callbacks).install(
            input,
            fresh,
            inputParams,
            params,
        )
        try { windowManager.addView(fresh, params) } catch (e: Exception) {
            Log.e(TAG, "Failed to add bubble view", e)
        }
        try { windowManager.addView(input, inputParams) } catch (e: Exception) {
            Log.e(TAG, "Failed to add bubble input view", e)
        }
        bubbleView = fresh
        touchView = input
        touchParams = inputParams
    }

}
