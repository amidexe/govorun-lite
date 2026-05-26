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
import android.util.Log
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityWindowInfo
import androidx.core.content.ContextCompat
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.util.AccessibilityFrameworkCrashGuard
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

        /**
         * Used by [com.govorun.lite.util.Haptics] to dispatch haptic
         * feedback through our floating bubble — that view is alive
         * whenever the bubble is on screen, regardless of whether a
         * caller has an Activity context handy.
         */
        fun getBubbleViewIfAttached(): View? = instance?.bubbleOverlay?.getView()
    }

    private var isImeVisible = false
    private var keyguardManager: KeyguardManager? = null
    private var bubbleOverlay: BubbleOverlayController? = null

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

    // Foreground app package — updated on TYPE_WINDOW_STATE_CHANGED, skipping
    // IME packages and our own. Used by the app filter to decide visibility.
    private var currentForegroundPackage: String? = null

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var vadRecorder: VadRecorder? = null
    @Volatile private var isVadActive = false
    private var isHoldMode = false   // true when recording started with useVad=false

    private var accessibilityInputMethod: LiteAccessibilityInputMethod? = null

    /** Debounces short SHOW transitions so that flicker bursts during
     *  window-focus reshuffles (e.g. leaving a search field via Home
     *  button — the search filter briefly lifts before the IME fully
     *  hides) don't paint a one-frame bubble visible-then-gone. Hide
     *  decisions are immediate; only show is delayed. 120 ms is long
     *  enough to absorb most transitions, short enough to feel
     *  responsive on a deliberate field tap. */
    private val showDebounceHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private val pendingShowRunnable = Runnable {
        bubbleOverlay?.setVisibility(android.view.View.VISIBLE)
    }

    // Accumulates sub-second speech durations from VAD callbacks. We only
    // store whole seconds in StatsStore (the UI shows minutes), but throwing
    // away every <1s segment would lose all the «ага», «понятно», «нет»-style
    // quick phrases — and the running counter would massively undercount
    // for users with chatty conversational dictation. Lives for the process
    // lifetime; on service restart we lose at most 999ms of remainder.
    @Volatile private var speechMillisRemainder: Long = 0L

    // Wall-clock auto-stop timer. Only armed when the user has opted into
    // Keep-Screen-On (without that, the system's own screen timeout already
    // ends the session via screenOffReceiver, well before any sane wall cap).
    // Default cap is 1 hour, configurable to 15 min / 1 hour / 3 hours / off.
    // Protects against forgetting + the music/podcast edge case where Silero
    // VAD treats vocals as speech and keeps the session alive indefinitely.
    private val autoStopHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private val autoStopRunnable = Runnable { stopVadRecording(silent = true) }

    // Speech-active grace timer. When VAD reports speech end we hold the
    // keep-screen-on override briefly so the screen doesn't sleep during
    // a natural inter-sentence pause. New speech inside the grace window
    // cancels this and keeps the screen up. Default-mode users (no opt-in
    // keep-screen flag) need this — without it Android's screen timeout
    // happily kills mid-utterance dictation since speech is not a
    // touchUserActivity event we have permission to fake.
    private val speechGraceHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private val speechGraceRunnable = Runnable {
        bubbleOverlay?.setKeepScreenOnOverride(false)
    }

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
        // Override the default to instantiate our subclass: it tracks
        // onStartInput / onFinishInput callback counts so pasteText can
        // detect zombie InputConnection state before committing.
        return LiteAccessibilityInputMethod(this).also { accessibilityInputMethod = it }
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onServiceConnected() {
        super.onServiceConnected()
        // Install before anything else: the framework can throw NPE inside
        // its own startInput / invalidateInput bookkeeping during this
        // very call sequence on some devices/Android versions, and we want
        // to be alive afterwards regardless. See the guard's class doc for
        // exactly which framework crashes are covered and why.
        AccessibilityFrameworkCrashGuard.install()
        instance = this
        Log.i(TAG, "Accessibility service connected")

        keyguardManager = getSystemService(KEYGUARD_SERVICE) as KeyguardManager

        bubbleOverlay = BubbleOverlayController(this, bubbleCallbacks).also {
            it.create(initiallyVisible = false)
        }

        registerReceiver(screenOffReceiver, IntentFilter(Intent.ACTION_SCREEN_OFF))
        registerReceiver(userPresentReceiver, IntentFilter(Intent.ACTION_USER_PRESENT))

        // Initial check — if the service starts while a keyboard is already up,
        // we'd otherwise wait for the next windows-changed event.
        bubbleOverlay?.getView()?.post { updateImeVisibility() }

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
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED -> {
                // Track which non-IME, non-self app is in the foreground so
                // the app filter has a reliable package to check against.
                // Skip system windows (copy toolbar, permission dialogs, system
                // overlays from "android" / "com.android.systemui") — they fire
                // TYPE_WINDOW_STATE_CHANGED too and would transiently clobber
                // currentForegroundPackage, causing the bubble to vanish.
                val pkg = event.packageName?.toString()
                if (pkg != null && !isImePackage(pkg) && pkg != packageName) {
                    val srcWindowType = try { event.source?.window?.type } catch (_: Exception) { null }
                    if (srcWindowType != AccessibilityWindowInfo.TYPE_SYSTEM) {
                        currentForegroundPackage = pkg
                    }
                }
                updateImeVisibility()
            }
            AccessibilityEvent.TYPE_WINDOWS_CHANGED,
            // typeViewFocused fires when the user moves focus between fields
            // inside the same window — e.g. login screen email -> password.
            // Without it, the IME-window event won't refire (the same IME stays
            // visible) and we won't know to hide the bubble for the new field.
            AccessibilityEvent.TYPE_VIEW_FOCUSED -> updateImeVisibility()
        }
    }

    private fun isImePackage(pkg: String): Boolean {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager
            ?: return false
        return imm.enabledInputMethodList.any { it.packageName == pkg }
    }

    private fun updateImeVisibility() {
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
        // Optional: hide on search fields too (Chrome address bar, app
        // search boxes, system search). Off by default — existing users
        // shouldn't suddenly find the bubble missing in places it used
        // to appear. Power users who want this enable it in Settings;
        // see InputFieldFilter.isSearchField for the detection logic.
        val searchField = Prefs.isHideInSearchEnabled(this) &&
            InputFieldFilter.isSearchField(accessibilityInputMethod)
        // Derive the foreground package live from the current window list
        // instead of relying on the cached value. The cache can be stale
        // during app transitions — the first TYPE_WINDOW_STATE_CHANGED on
        // return often comes from the launcher/animation, not the target app,
        // so the cache holds the wrong package when we reach this point.
        val livePkg = try {
            windows?.mapNotNull { w ->
                if (w.type != AccessibilityWindowInfo.TYPE_APPLICATION) return@mapNotNull null
                val pkg = w.root?.packageName?.toString() ?: return@mapNotNull null
                if (pkg == packageName || isImePackage(pkg)) return@mapNotNull null
                pkg
            }?.firstOrNull()
        } catch (_: Exception) { null }
        if (livePkg != null) currentForegroundPackage = livePkg
        val appFiltered = Prefs.isAppFiltered(this, currentForegroundPackage)
        val shouldShow = imeVisible && !locked && !passwordField && !searchField && !appFiltered

        // In always-show mode the bubble stays on screen even when no IME
        // is up — the user explicitly opted into "voice-only" workflow via
        // the Quick Settings tile. Recording still stops on real focus loss
        // so the mic isn't held forever, but the bubble itself sticks.
        val visibilityChanged = shouldShow != isImeVisible
        if (!visibilityChanged && !manualAlwaysShow) return
        if (visibilityChanged && !shouldShow) stopVadRecording(silent = true)
        isImeVisible = shouldShow

        // Silence override beats everything — if the user explicitly put
        // the bird to sleep, keep it hidden no matter what. Always-show
        // also intentionally bypasses the search filter: tile is the
        // user's explicit "I want voice everywhere right now" toggle.
        val effectiveVisibility = when {
            manualSilence -> View.GONE
            manualAlwaysShow && !locked && !passwordField -> View.VISIBLE
            shouldShow -> View.VISIBLE
            else -> View.GONE
        }
        applyVisibilityWithDebounce(effectiveVisibility)
        AppLog.log(this, "Service: bubbleShow=$shouldShow ime=$imeVisible locked=$locked password=$passwordField search=$searchField appFiltered=$appFiltered alwaysShow=$manualAlwaysShow silence=$manualSilence foreground=$currentForegroundPackage")
    }

    /**
     * Apply a new visibility to the bubble. Hides happen immediately;
     * shows are debounced by ~120 ms so that brief transient SHOW
     * decisions during window-focus shuffles (the search-field bug:
     * leaving search field for Home momentarily flips !searchField=true
     * before imeVisible flips to false) don't paint a single-frame
     * bubble flicker.
     */
    private fun applyVisibilityWithDebounce(visibility: Int) {
        showDebounceHandler.removeCallbacks(pendingShowRunnable)
        if (visibility == View.VISIBLE) {
            showDebounceHandler.postDelayed(pendingShowRunnable, 120L)
        } else {
            bubbleOverlay?.setVisibility(visibility)
        }
    }

    /** Lazily created on first IME bind — TextInserter needs the service
     *  reference to compile-time but the IME instance only after Android
     *  calls onCreateInputMethod, so we look it up via lambda each time. */
    private val textInserter by lazy {
        TextInserter(this) { accessibilityInputMethod }
    }

    private fun pasteText(text: String) = textInserter.insert(text)

    @SuppressLint("MissingPermission")
    private fun startVadRecording(useVad: Boolean = true) {
        if (isVadActive) return
        isHoldMode = !useVad
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
        bubbleOverlay?.setRecording(true)
        // In hold-to-talk mode VAD is disabled so onSpeechActive never fires —
        // the keep-screen-on override must be set here directly. In VAD mode
        // the override is driven by speech-detection callbacks instead.
        if (!useVad) bubbleOverlay?.setKeepScreenOnOverride(true)
        scheduleAutoStop()

        // Build + start the recorder SYNCHRONOUSLY on the main thread. Dispatching
        // through scope.launch added ~50-200 ms of latency that swallowed the first
        // word on fast taps. The recorder opens AudioRecord + startRecording() inline,
        // then spawns its own I/O coroutine for VAD reading and transcription.
        val recorder = VadRecorder(this).also { vadRecorder = it }
        recorder.start(
            scope = scope,
            transcriberProvider = { OfflineTranscriber.getInstance(this@LiteAccessibilityService) },
            onSegment = { text -> pasteText(text) },
            onDone = { hadText ->
                if (isHoldMode) {
                    // Always clear processing state, even if nothing was recognised —
                    // otherwise the yellow spinning bubble gets stuck on silence.
                    bubbleOverlay?.setProcessing(false)
                    bubbleOverlay?.setKeepScreenOnOverride(false)
                }
                if (hadText && Prefs.isVoiceSuffixEnabled(this@LiteAccessibilityService)) {
                    pasteText("\n\n[текст распознан с голоса]")
                }
            },
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
            onSpeechActive = { active ->
                if (active) {
                    speechGraceHandler.removeCallbacks(speechGraceRunnable)
                    bubbleOverlay?.setKeepScreenOnOverride(true)
                } else {
                    // 5 s grace lets the screen stay alive across natural
                    // inter-sentence pauses without dragging on after the
                    // user has clearly finished and walked away.
                    speechGraceHandler.removeCallbacks(speechGraceRunnable)
                    speechGraceHandler.postDelayed(speechGraceRunnable, 5_000L)
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
        autoStopHandler.removeCallbacks(autoStopRunnable)
        speechGraceHandler.removeCallbacks(speechGraceRunnable)
        if (!silent) vibrateStop()
        bubbleOverlay?.setRecording(false)
        if (isHoldMode) {
            // Hold-to-talk: transcription of the full buffer can take several
            // seconds for long recordings. Show the processing state so the
            // user knows the app is working, not frozen. Keep the screen alive
            // during that time too — both are cleared in the onDone callback.
            bubbleOverlay?.setProcessing(true)
        } else {
            bubbleOverlay?.setKeepScreenOnOverride(false)
        }
        Log.i(TAG, "VAD recording stopped")
    }

    /** Arm the wall-clock cap timer if the user is in Keep-Screen-On mode
     *  with a finite cap selected. Pure no-op for the default (off) mode —
     *  the screenOffReceiver handles that path via system screen timeout.
     *  Each branch writes a positive AppLog line so a 30-second tap-stop
     *  smoke test can confirm in the log which path actually ran (instead
     *  of waiting 15 minutes to see whether the cap stops the session). */
    private fun scheduleAutoStop() {
        autoStopHandler.removeCallbacks(autoStopRunnable)
        if (!Prefs.isKeepScreenOnEnabled(this)) {
            AppLog.log(this, "Service: auto-stop SKIPPED (keep-screen-on = false)")
            return
        }
        val minutes = Prefs.getAutoStopMinutes(this)
        if (minutes <= 0) {
            AppLog.log(this, "Service: auto-stop SKIPPED (cap = unlimited)")
            return
        }
        val delayMs = minutes.toLong() * 60_000L
        autoStopHandler.postDelayed(autoStopRunnable, delayMs)
        AppLog.log(this, "Service: auto-stop ARMED for $minutes min")
    }

    // Recording start = a deliberate "this is happening now" haptic;
    // recording stop = double-confirm matching Telegram's voice-message
    // release feel. Both go through Haptics → View.performHapticFeedback
    // with FLAG_IGNORE_GLOBAL_SETTING so the user can't accidentally mute
    // them via the system Touch-feedback toggle.
    private fun vibrateStart() = Haptics.longPress(this)
    private fun vibrateStop() = Haptics.doubleTap(this)

    override fun onInterrupt() {}

    /** Glue between BubbleOverlayController's gesture detection and the
     *  service's recording lifecycle. Hold-to-talk uses raw-PCM mode so
     *  the whole utterance reaches GigaAM in one contextual pass instead
     *  of being split by VAD on internal pauses. */
    private val bubbleCallbacks = object : BubbleOverlayController.Callbacks {
        override fun onTap() {
            if (isVadActive) stopVadRecording() else startVadRecording()
        }
        override fun onHoldStart() = startVadRecording(useVad = false)
        override fun onHoldStop(silent: Boolean) = stopVadRecording(silent = silent)
        override fun isVadActive(): Boolean = this@LiteAccessibilityService.isVadActive
    }

    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        // Wallpaper colour changes arrive here on API 31+. Rebuild the bubble
        // with a fresh DC-wrapped context so the new accent takes effect
        // without the user having to toggle the service.
        bubbleOverlay?.rebuild(initiallyVisible = isImeVisible)
    }

    /**
     * Called from Settings after the user moves the transparency slider — the
     * existing BubbleView picks up the new alpha immediately; no rebuild.
     */
    fun applyBubbleAlphaFromPrefs() {
        bubbleOverlay?.applyAlpha()
    }

    /**
     * Called from Settings after the user moves the size slider. Size changes
     * affect onMeasure() output, so we rebuild the bubble (same path as the
     * wallpaper-colour reaction) instead of trying to re-layout in place.
     */
    fun applyBubbleSizeFromPrefs() {
        bubbleOverlay?.applySize(initiallyVisible = isImeVisible)
    }

    /**
     * True when the bubble is currently painted on screen.
     */
    fun isBubbleVisible(): Boolean = bubbleOverlay?.isVisible() == true

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
        val locked = keyguardManager?.isKeyguardLocked == true
        val passwordField = InputFieldFilter.isPasswordField(accessibilityInputMethod)
        val effective = when {
            manualSilence -> View.GONE
            locked || passwordField -> View.GONE
            manualAlwaysShow -> View.VISIBLE
            isImeVisible -> View.VISIBLE
            else -> View.GONE
        }
        bubbleOverlay?.setVisibility(effective)
    }

    /**
     * Called from Settings when the user picks left/right side. Updates the
     * window-manager gravity in-place so the bubble flies to the new edge
     * without a rebuild.
     */
    fun applyBubbleSideFromPrefs() {
        bubbleOverlay?.applySide()
    }

    override fun onDestroy() {
        instance = null
        stopVadRecording(silent = true)
        scope.cancel()
        try { unregisterReceiver(screenOffReceiver) } catch (_: Exception) {}
        try { unregisterReceiver(userPresentReceiver) } catch (_: Exception) {}
        bubbleOverlay?.destroy()
        bubbleOverlay = null
        super.onDestroy()
    }
}
