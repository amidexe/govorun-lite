package com.govorun.lite.service

import android.Manifest
import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.InputMethod
import android.annotation.SuppressLint
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.view.HapticFeedbackConstants
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityWindowInfo
import androidx.appcompat.view.ContextThemeWrapper
import androidx.core.content.ContextCompat
import com.google.android.material.color.DynamicColors
import com.govorun.lite.R
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.transcriber.Dictionary
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.Haptics
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
        // Press duration after which a touch is treated as "hold-to-talk"
        // instead of a tap-toggle. 250 ms is comfortably longer than a
        // normal tap (~50–100 ms) and shorter than a deliberate long-press.
        private const val HOLD_DELAY_MS = 250L
        // Smaller-than-drag slop used to detect *intentional* finger
        // movement during the hold-wait window — without it, slow back-edge
        // swipes (which haven't crossed the 10dp drag threshold yet by the
        // time HOLD_DELAY_MS elapses) trigger a brief recording flash.
        private const val HOLD_MOVEMENT_SLOP_DP = 5f

        /**
         * Used by [com.govorun.lite.util.Haptics] to dispatch haptic
         * feedback through our floating bubble — that view is alive
         * whenever the bubble is on screen, regardless of whether a
         * caller has an Activity context handy.
         */
        fun getBubbleViewIfAttached(): View? = instance?.bubbleView
    }

    private var isImeVisible = false
    private var bubbleView: BubbleView? = null
    private var windowManager: WindowManager? = null
    private var keyguardManager: KeyguardManager? = null
    private var bubbleParams: WindowManager.LayoutParams? = null

    // User-controlled "always show" override, toggled via the Quick Settings
    // tile. Runtime-only (not persisted) — service restart resets to false.
    // When true, updateImeVisibility() forces the bubble visible regardless
    // of IME state — useful when the user wants to dictate without the
    // keyboard taking up half the screen (Notion, Google Docs, text editors
    // with their own toolbar). See toggleAlwaysShow().
    @Volatile private var manualAlwaysShow: Boolean = false

    // User-controlled "silence" override, toggled via a separate Quick
    // Settings tile (the «sleeping bird» one). When true, the bubble stays
    // hidden even if an IME is up — the metaphor is «voice input is asleep»,
    // useful at night, in meetings, in libraries. Runtime-only; resets on
    // service restart. Symmetrical toggle (one tap on, one tap off) — does
    // NOT actually disable the AccessibilityService, just hides the bubble.
    @Volatile private var manualSilence: Boolean = false

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var vadRecorder: VadRecorder? = null
    @Volatile private var isVadActive = false

    private var accessibilityInputMethod: InputMethod? = null

    // Accumulates sub-second speech durations from VAD callbacks. We only
    // store whole seconds in StatsStore (the UI shows minutes), but throwing
    // away every <1s segment would lose all the «ага», «понятно», «нет»-style
    // quick phrases — and the running counter would massively undercount
    // for users with chatty conversational dictation. Lives for the process
    // lifetime; on service restart we lose at most 999ms of remainder.
    @Volatile private var speechMillisRemainder: Long = 0L

    // Was a wrapper around View.performHapticFeedback with predefined HID
    // constants — see util/Haptics.kt for why we abandoned that approach
    // (silent no-op on Xiaomi/Huawei). Kept as no-op signature briefly so
    // call sites compile while we migrate; both callers are below.

    private val screenOffReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == Intent.ACTION_SCREEN_OFF) stopVadRecording(silent = true)
        }
    }

    // Re-evaluates bubble visibility when the user unlocks the device. Without
    // this, if a keyboard was already up on the lockscreen (e.g. password
    // entry) we'd correctly hide the bubble, but on unlock — when the same IME
    // window stays focused (notification reply, app behind keyguard) — the
    // accessibility windows-changed event may not refire, leaving the bubble
    // hidden until the next focus change.
    private val userPresentReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == Intent.ACTION_USER_PRESENT) updateImeVisibility()
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
        keyguardManager = getSystemService(KEYGUARD_SERVICE) as KeyguardManager

        bubbleParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
            x = 16
            // Restore the user's last drag position. Without this, every
            // time Android restarts our service (memory pressure, OEM
            // battery-saver killing the process, accessibility unbind/rebind
            // cycle) the bubble snaps back to vertical centre.
            y = Prefs.getBubbleY(this@LiteAccessibilityService)
        }

        attachFreshBubble(initiallyVisible = false)

        registerReceiver(screenOffReceiver, IntentFilter(Intent.ACTION_SCREEN_OFF))
        registerReceiver(userPresentReceiver, IntentFilter(Intent.ACTION_USER_PRESENT))

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
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED,
            // typeViewFocused fires when the user moves focus between fields
            // inside the same window — e.g. login screen email -> password.
            // Without it, the IME-window event won't refire (the same IME stays
            // visible) and we won't know to hide the bubble for the new field.
            AccessibilityEvent.TYPE_VIEW_FOCUSED -> updateImeVisibility(event.packageName?.toString())
        }
    }

    private fun updateImeVisibility(pkg: String? = null) {
        val imeVisible = try {
            windows?.any { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD } == true
        } catch (e: Exception) {
            false
        }
        // Hide on keyguard even if an IME is technically visible (lockscreen
        // password entry, notification reply over keyguard). The bubble there
        // is unwanted noise — the user can't dictate into a password field
        // anyway, and overlay UI on the lockscreen erodes trust regardless of
        // whether we actually read the field (we don't).
        val locked = keyguardManager?.isKeyguardLocked == true
        // Hide on password fields anywhere in any app. Detection lives in
        // InputFieldFilter — see that file for the full rationale.
        val passwordField = InputFieldFilter.isPasswordField(accessibilityInputMethod)
        val shouldShow = imeVisible && !locked && !passwordField

        // In always-show mode the bubble stays on screen even when no IME
        // is up — the user explicitly opted into "voice-only" workflow via
        // the Quick Settings tile. Recording still stops on real focus loss
        // so the mic isn't held forever, but the bubble itself sticks.
        val visibilityChanged = shouldShow != isImeVisible
        if (!visibilityChanged && !manualAlwaysShow) return
        if (visibilityChanged && !shouldShow) stopVadRecording(silent = true)
        isImeVisible = shouldShow

        // Silence override beats everything — if the user explicitly put
        // the bird to sleep, keep it hidden no matter what.
        val effectiveVisibility = when {
            manualSilence -> View.GONE
            manualAlwaysShow && !locked && !passwordField -> View.VISIBLE
            shouldShow -> View.VISIBLE
            else -> View.GONE
        }
        bubbleView?.post { bubbleView?.visibility = effectiveVisibility }
        AppLog.log(this, "Service: bubbleShow=$shouldShow ime=$imeVisible locked=$locked password=$passwordField alwaysShow=$manualAlwaysShow silence=$manualSilence pkg=$pkg")
    }

    private fun pasteText(text: String) {
        if (text.isBlank()) return
        // Apply user dictionary BEFORE commitText. This is a lexical
        // post-pass — fixing jargon, abbreviations, proper-noun spellings
        // the recogniser doesn't know. Empty dictionary returns the text
        // unchanged, so this costs nothing for users who don't set one up.
        val replaced = Dictionary.applyReplacements(this, text)
        if (replaced != text) {
            AppLog.log(this, "Dictionary applied: '${text.take(40)}' → '${replaced.take(40)}'")
        }
        val connection = accessibilityInputMethod?.currentInputConnection
        if (connection == null) {
            AppLog.log(this, "Paste: InputConnection=null — dropping '${replaced.take(40)}'")
            return
        }
        val spacedText = prependSpaceIfNeeded(replaced, connection)
        connection.commitText(spacedText, 1, null)
        AppLog.log(this, "Paste: commitText len=${spacedText.length}")
        StatsStore.addWords(this, StatsStore.countWords(replaced))
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
    private fun startVadRecording(useVad: Boolean = true) {
        if (isVadActive) return
        // Mic permission can disappear out from under us — most commonly when
        // the user picked "Только в этот раз" and Android quietly revoked it
        // (and killed the previous process). MainActivity already has a card
        // with a "open app settings" button for exactly this case — open it
        // directly so the user gets unmistakable full-screen feedback.
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED) {
            AppLog.log(this, "Service: tap with revoked RECORD_AUDIO → opening MainActivity")
            val intent = Intent(this, MainActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            }
            try { startActivity(intent) } catch (e: Exception) {
                Log.w(TAG, "Failed to open MainActivity from bubble tap: ${e.message}")
            }
            return
        }
        if (!GigaAmModel.isInstalled(this)) {
            Log.w(TAG, "GigaAM model not installed — cannot start recording")
            AppLog.log(this, "Service: start attempt, but model not installed")
            return
        }
        AppLog.log(this, "Service: VAD start (useVad=$useVad)")
        isVadActive = true
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
            // Per-segment speech time in milliseconds. Replaces the old
            // "session length" counter that added tap-to-tap elapsed time
            // (including silence and forgotten-on records). We accumulate
            // millisecond fractions across segments before rolling whole
            // seconds into StatsStore — otherwise short utterances like
            // «ага» (~700ms) would round to zero and disappear, and the
            // displayed minutes would still undercount by a lot.
            onSpeechMillis = { millis ->
                speechMillisRemainder += millis
                val whole = speechMillisRemainder / 1000L
                if (whole > 0L) {
                    StatsStore.addSeconds(this@LiteAccessibilityService, whole)
                    speechMillisRemainder -= whole * 1000L
                }
            },
            useVad = useVad,
        )
        Log.i(TAG, "VAD recording started")
    }

    private fun stopVadRecording(silent: Boolean = false) {
        if (!isVadActive) return
        isVadActive = false
        // Speech time is accounted per VAD segment via the onSpeechMillis
        // callback above — we don't add anything here. Any sub-second
        // remainder in speechMillisRemainder (<1000ms) is lost on stop;
        // accepted, since the alternative (carrying it across sessions)
        // would couple the counter to service lifecycle in a way users
        // would find confusing.
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

    // Recording start = a deliberate "this is happening now" haptic;
    // recording stop = double-confirm matching Telegram's voice-message
    // release feel. Both go through Haptics → View.performHapticFeedback
    // with FLAG_IGNORE_GLOBAL_SETTING so the user can't accidentally mute
    // them via the system Touch-feedback toggle.
    private fun vibrateStart() = Haptics.longPress(this)
    private fun vibrateStop() = Haptics.doubleTap(this)

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
        val holdMovementSlopPx = HOLD_MOVEMENT_SLOP_DP * resources.displayMetrics.density
        fresh.setOnTouchListener(object : View.OnTouchListener {
            private var initialY = 0
            private var initialTouchY = 0f
            private var initialTouchX = 0f
            private var lastTouchX = 0f
            private var lastTouchY = 0f
            private var dragged = false
            private var holdStarted = false
            private val holdHandler = android.os.Handler(android.os.Looper.getMainLooper())
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
                // Hold-to-talk uses raw-PCM mode: the whole utterance goes to
                // GigaAM in one shot so phrases with internal pauses stay as a
                // single contextual recognition, instead of being split by VAD
                // and concatenated.
                startVadRecording(useVad = false)
            }
            override fun onTouch(v: View, event: MotionEvent): Boolean {
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        initialY = bubbleParams!!.y
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
                                bubbleParams!!.y = initialY + dy.toInt()
                                windowManager?.updateViewLayout(bubbleView, bubbleParams)
                            }
                        }
                        return true
                    }
                    MotionEvent.ACTION_UP -> {
                        holdHandler.removeCallbacks(holdRunnable)
                        if (dragged) {
                            // Persist final Y so the bubble lands in the same
                            // place after the next service restart.
                            Prefs.setBubbleY(this@LiteAccessibilityService, bubbleParams!!.y)
                            return true
                        }
                        if (holdStarted) {
                            // Hold-to-talk: releasing the finger stops the
                            // recording (and triggers VAD pipeline → paste).
                            stopVadRecording()
                            return true
                        }
                        // Quick tap before HOLD_DELAY_MS — same toggle
                        // behaviour we've always had (start, then tap again
                        // to stop).
                        if (isVadActive) stopVadRecording() else startVadRecording()
                        return true
                    }
                    MotionEvent.ACTION_CANCEL -> {
                        // CANCEL means the gesture was aborted by the system
                        // — usually because the finger left the view bounds
                        // mid-touch (back-edge swipe across the bubble is
                        // the typical cause). Do NOT treat this as a tap or
                        // hold-release; just clean up. Without this branch,
                        // any swipe over the bubble triggered a phantom
                        // recording start because CANCEL fell through to the
                        // toggle path.
                        holdHandler.removeCallbacks(holdRunnable)
                        if (holdStarted) stopVadRecording(silent = true)
                        if (dragged) Prefs.setBubbleY(
                            this@LiteAccessibilityService,
                            bubbleParams!!.y
                        )
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

    /**
     * Called from Settings after the user moves the size slider. Size changes
     * affect onMeasure() output, so we rebuild the bubble (same path as the
     * wallpaper-colour reaction) instead of trying to re-layout in place.
     */
    fun applyBubbleSizeFromPrefs() {
        attachFreshBubble(initiallyVisible = isImeVisible)
    }

    /**
     * True when the bubble is currently painted on screen.
     */
    fun isBubbleVisible(): Boolean = bubbleView?.visibility == View.VISIBLE

    /**
     * Quick Settings Tile read-out. True when the user has opted into
     * always-show mode (bubble stays on screen even without an active IME).
     * Default is false (standard behaviour: bubble follows the keyboard).
     */
    fun isAlwaysShowEnabled(): Boolean = manualAlwaysShow

    /**
     * Quick Settings Tile action. Toggles the always-show mode. When turned
     * on, the bubble stays visible even if there's no active text field —
     * lets the user dictate while the keyboard is collapsed (more screen
     * real estate for the actual text). When turned off, standard behaviour
     * resumes (bubble appears with the IME, hides when it goes away).
     *
     * Runtime-only flag; resets on service restart. Locked-screen and
     * password-field protections still apply — we don't paint the bubble
     * over keyguard or password prompts even in always-show mode.
     */
    fun toggleAlwaysShow() {
        manualAlwaysShow = !manualAlwaysShow
        applyVisibilityNow()
    }

    /** Quick Settings Tile read-out for the silence tile. */
    fun isSilenceEnabled(): Boolean = manualSilence

    /**
     * Quick Settings Tile action for the «sleeping bird» tile. When turned
     * on, the bubble stays hidden even if an IME is up — voice input is
     * asleep until the user taps the tile again. Symmetrical: one tap to
     * silence, one tap to wake. Doesn't disable the service.
     */
    fun toggleSilence() {
        manualSilence = !manualSilence
        applyVisibilityNow()
    }

    /**
     * Recomputes effective bubble visibility from current flags + IME state
     * and applies it on the view's looper. Used by both tile toggles so the
     * shade tap is visually instant rather than waiting for the next IME
     * event.
     */
    private fun applyVisibilityNow() {
        bubbleView?.post {
            val locked = keyguardManager?.isKeyguardLocked == true
            val passwordField = InputFieldFilter.isPasswordField(accessibilityInputMethod)
            bubbleView?.visibility = when {
                manualSilence -> View.GONE
                locked || passwordField -> View.GONE
                manualAlwaysShow -> View.VISIBLE
                isImeVisible -> View.VISIBLE
                else -> View.GONE
            }
        }
    }

    /**
     * Called from Settings when the user picks left/right side. Updates the
     * window-manager gravity in-place so the bubble flies to the new edge
     * without a rebuild.
     */
    fun applyBubbleSideFromPrefs() {
        val params = bubbleParams ?: return
        params.gravity = bubbleHorizontalGravity() or Gravity.CENTER_VERTICAL
        try { windowManager?.updateViewLayout(bubbleView, params) } catch (_: Exception) {}
    }

    private fun bubbleHorizontalGravity(): Int =
        if (Prefs.getBubbleSide(this) == Prefs.BUBBLE_SIDE_LEFT) Gravity.START else Gravity.END

    override fun onDestroy() {
        instance = null
        stopVadRecording(silent = true)
        scope.cancel()
        try { unregisterReceiver(screenOffReceiver) } catch (_: Exception) {}
        try { unregisterReceiver(userPresentReceiver) } catch (_: Exception) {}
        bubbleView?.let { windowManager?.removeView(it) }
        super.onDestroy()
    }
}
