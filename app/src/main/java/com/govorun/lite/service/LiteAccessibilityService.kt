package com.govorun.lite.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.InputMethod
import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.PixelFormat
import android.os.SystemClock
import android.view.HapticFeedbackConstants
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityWindowInfo
import androidx.appcompat.view.ContextThemeWrapper
import com.google.android.material.color.DynamicColors
import com.govorun.lite.R
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * Lite accessibility service: one mode, VAD only.
 *  - Shows Говорун only while the soft keyboard (IME) is visible. That's the
 *    most reliable "user is typing" signal across all apps — text-field focus
 *    events are unreliable in WebView/Compose/custom EditText, but the IME
 *    only ever appears when the system itself thinks an editable target is
 *    active, so this avoids false positives in players, games, etc.
 *  - Tap Говорун to start VAD recording; pauses produce paragraph breaks;
 *    tap again to stop.
 *  - Inserts text into the focused field via InputMethod commitText
 *    (requires minSdk 33 + accessibility_config flagInputMethodEditor).
 */
class LiteAccessibilityService : AccessibilityService() {

    companion object {
        var instance: LiteAccessibilityService? = null
            private set
        private const val TAG = "LiteAccessibility"
        private const val DRAG_THRESHOLD_DP = 10f
    }

    private var isImeVisible = false
    private var bubbleView: BubbleView? = null
    private var windowManager: WindowManager? = null
    private var bubbleParams: WindowManager.LayoutParams? = null

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var vadRecorder: VadRecorder? = null
    @Volatile private var isVadActive = false

    private var accessibilityInputMethod: InputMethod? = null
    private var vadStartElapsedMs: Long = 0L

    @Suppress("DEPRECATION")
    private fun haptic(type: Int) {
        if (!Prefs.isHapticsEnabled(this)) return
        bubbleView?.performHapticFeedback(type, HapticFeedbackConstants.FLAG_IGNORE_GLOBAL_SETTING)
    }

    private val screenOffReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == Intent.ACTION_SCREEN_OFF) stopVadRecording(silent = true)
        }
    }

    override fun onCreateInputMethod(): InputMethod {
        return super.onCreateInputMethod().also { accessibilityInputMethod = it }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        Log.i(TAG, "Accessibility service connected")

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        bubbleParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.END or Gravity.CENTER_VERTICAL
            x = 16
        }

        attachFreshBubble(initiallyVisible = false)

        registerReceiver(screenOffReceiver, IntentFilter(Intent.ACTION_SCREEN_OFF))

        // Initial check — if the service starts while a keyboard is already up,
        // we'd otherwise wait for the next windows-changed event.
        bubbleView?.post { updateImeVisibility() }

        // Warm up the GigaAM recognizer and Silero VAD in the background so the
        // first user tap doesn't race against model load. Without this, the
        // first recording after service connect consistently lost its audio —
        // ONNX session init can burn 100-500 ms, during which the bubble was
        // already red but the mic hadn't started capturing yet.
        if (GigaAmModel.isInstalled(this)) {
            scope.launch(Dispatchers.IO) {
                try {
                    OfflineTranscriber.getInstance(this@LiteAccessibilityService)
                    VadRecorder.warmUp(this@LiteAccessibilityService)
                    Log.i(TAG, "Transcriber + VAD pre-warmed")
                } catch (e: Exception) {
                    Log.w(TAG, "Pre-warm failed: ${e.message}")
                }
            }
        }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        event ?: return
        when (event.eventType) {
            AccessibilityEvent.TYPE_WINDOWS_CHANGED,
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED -> updateImeVisibility(event.packageName?.toString())
        }
    }

    private fun updateImeVisibility(pkg: String? = null) {
        val visible = try {
            windows?.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD } == true
        } catch (e: Exception) {
            false
        }
        if (visible == isImeVisible) return
        isImeVisible = visible
        if (!visible) stopVadRecording(silent = true)
        bubbleView?.post {
            bubbleView?.visibility = if (visible) View.VISIBLE else View.GONE
        }
        AppLog.log(this, "Service: imeVisible=$visible pkg=$pkg")
    }

    private fun pasteText(text: String) {
        if (text.isBlank()) return
        val connection = accessibilityInputMethod?.currentInputConnection
        if (connection == null) {
            AppLog.log(this, "Paste: InputConnection=null — dropping '${text.take(40)}'")
            return
        }
        val spacedText = prependSpaceIfNeeded(text, connection)
        connection.commitText(spacedText, 1, null)
        AppLog.log(this, "Paste: commitText len=${spacedText.length}")
        StatsStore.addWords(this, StatsStore.countWords(text))
    }

    private fun prependSpaceIfNeeded(
        text: String,
        connection: InputMethod.AccessibilityInputConnection
    ): String {
        val surrounding = connection.getSurroundingText(1, 0, 0)
        val lastChar = surrounding?.text?.toString()?.lastOrNull()
        return if (lastChar != null && !lastChar.isWhitespace()) " $text" else text
    }

    @SuppressLint("MissingPermission")
    private fun startVadRecording() {
        if (isVadActive) return
        if (!GigaAmModel.isInstalled(this)) {
            Log.w(TAG, "GigaAM model not installed — cannot start recording")
            AppLog.log(this, "Service: start attempt, but model not installed")
            return
        }
        AppLog.log(this, "Service: VAD start")
        isVadActive = true
        vadStartElapsedMs = SystemClock.elapsedRealtime()
        vibrateStart()
        bubbleView?.setRecording(true)
        bubbleParams?.flags = bubbleParams!!.flags or WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
        windowManager?.updateViewLayout(bubbleView, bubbleParams)

        // Build + start the recorder SYNCHRONOUSLY on the main thread. Dispatching
        // through scope.launch added ~50-200 ms of latency that swallowed the first
        // word on fast taps. The recorder opens AudioRecord + startRecording() inline,
        // then spawns its own I/O coroutine for VAD reading and transcription.
        val recorder = VadRecorder(this).also { vadRecorder = it }
        recorder.start(
            scope = scope,
            transcriberProvider = { OfflineTranscriber.getInstance(this@LiteAccessibilityService) },
            onSegment = { text -> pasteText(text) },
        )
        Log.i(TAG, "VAD recording started")
    }

    private fun stopVadRecording(silent: Boolean = false) {
        if (!isVadActive) return
        isVadActive = false
        val elapsedSec = (SystemClock.elapsedRealtime() - vadStartElapsedMs) / 1000L
        StatsStore.addSeconds(this, elapsedSec)
        // Signal the recorder to exit its read loop. Its finally block will flush
        // VAD's buffer and send any tail audio through the transcriber pipeline
        // before releasing the mic, so the last phrase isn't lost on quick taps.
        vadRecorder?.stop()
        vadRecorder = null
        if (!silent) vibrateStop()
        bubbleView?.setRecording(false)
        bubbleParams?.flags = bubbleParams!!.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON.inv()
        windowManager?.updateViewLayout(bubbleView, bubbleParams)
        Log.i(TAG, "VAD recording stopped")
    }

    private fun vibrateStart() {
        haptic(HapticFeedbackConstants.VIRTUAL_KEY)
    }

    private fun vibrateStop() {
        haptic(if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R)
            HapticFeedbackConstants.REJECT else HapticFeedbackConstants.LONG_PRESS)
    }

    override fun onInterrupt() {}

    /**
     * Services don't get DynamicColors applied automatically (that helper is
     * activity-scoped). Wrapping explicitly here is what makes the bubble
     * pick up the user's wallpaper accent — without this, colorPrimary*
     * resolves to the static M3 defaults and the bubble reads as grey even
     * when Gboard/System UI are clearly using the dynamic palette.
     */
    private fun bubbleContext(): Context {
        val themed = ContextThemeWrapper(this, R.style.Theme_GovorunLite)
        return DynamicColors.wrapContextIfAvailable(themed)
    }

    /**
     * Reinstall a fresh BubbleView — used on first connect and whenever we
     * need to rebind to a DC-wrapped context (wallpaper colour change,
     * transparency pref change triggered from Settings via refreshBubble()).
     */
    @SuppressLint("ClickableViewAccessibility")
    private fun attachFreshBubble(initiallyVisible: Boolean) {
        val wm = windowManager ?: return
        bubbleView?.let {
            try { wm.removeView(it) } catch (_: Exception) {}
        }
        val fresh = BubbleView(bubbleContext()).apply {
            setIdleAlpha(Prefs.getBubbleAlpha(this@LiteAccessibilityService))
            visibility = if (initiallyVisible) View.VISIBLE else View.GONE
        }
        val dragThresholdPx = DRAG_THRESHOLD_DP * resources.displayMetrics.density
        fresh.setOnTouchListener(object : View.OnTouchListener {
            private var initialY = 0
            private var initialTouchY = 0f
            private var dragged = false
            override fun onTouch(v: View, event: MotionEvent): Boolean {
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        initialY = bubbleParams!!.y
                        initialTouchY = event.rawY
                        dragged = false
                        return true
                    }
                    MotionEvent.ACTION_MOVE -> {
                        if (Math.abs(event.rawY - initialTouchY) > dragThresholdPx) {
                            dragged = true
                            bubbleParams!!.y = initialY + (event.rawY - initialTouchY).toInt()
                            windowManager?.updateViewLayout(bubbleView, bubbleParams)
                        }
                        return true
                    }
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                        if (dragged) return true
                        if (isVadActive) stopVadRecording() else startVadRecording()
                        return true
                    }
                }
                return false
            }
        })
        try { wm.addView(fresh, bubbleParams) } catch (e: Exception) {
            Log.e(TAG, "Failed to add bubble view", e)
        }
        bubbleView = fresh
    }

    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        // Wallpaper colour changes arrive here on API 31+. Rebuild the bubble
        // with a fresh DC-wrapped context so the new accent takes effect
        // without the user having to toggle the service.
        attachFreshBubble(initiallyVisible = isImeVisible)
    }

    /**
     * Called from Settings after the user moves the transparency slider — the
     * existing BubbleView picks up the new alpha immediately; no rebuild.
     */
    fun applyBubbleAlphaFromPrefs() {
        bubbleView?.setIdleAlpha(Prefs.getBubbleAlpha(this))
    }

    override fun onDestroy() {
        instance = null
        stopVadRecording(silent = true)
        scope.cancel()
        try { unregisterReceiver(screenOffReceiver) } catch (_: Exception) {}
        bubbleView?.let { windowManager?.removeView(it) }
        super.onDestroy()
    }
}
