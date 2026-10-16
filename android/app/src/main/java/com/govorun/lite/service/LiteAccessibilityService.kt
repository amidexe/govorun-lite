package com.govorun.lite.service

import android.Manifest
import android.accessibilityservice.AccessibilityService
import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.util.Log
import android.view.View
import android.view.accessibility.AccessibilityEvent
import androidx.core.content.ContextCompat
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.officialStylizationActive
import com.govorun.lite.diagnostics.AppLifecycleDiagnostics
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.stats.AndroidStatsRecorder
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.stylization.StylizationCoordinator
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.PendingMicrophoneCapture
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.ui.MainActivity
import com.govorun.lite.util.AccessibilityFrameworkCrashGuard
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.Haptics
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
/**
 * Accessibility dictation service shown while the IME is visible. Legacy
 * mode inserts pause-split local recognition immediately. Contextual tap and
 * hold modes collect those fragments, then stylize and insert the complete
 * phrase after confirmation. Text insertion uses InputMethod.commitText on
 * Android 13+ with an accessibility-node fallback.
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

    private var bubbleOverlay: BubbleOverlayController? = null

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var vadRecorder: VadRecorder? = null
    private val pendingMicrophone by lazy { PendingMicrophoneCapture(this, scope) }
    @Volatile private var isVadActive = false
    private var recordingUiMode = BubbleOverlayController.RecordingUiMode.LEGACY
    private var contextualTranscript: TranscriptDeliverySession? = null
    private var dictationTiming: DictationTiming? = null
    private val visibilityController by lazy {
        BubbleVisibilityController(
            service = this,
            overlay = { bubbleOverlay },
            isRecording = { isVadActive },
            recordingMode = { recordingUiMode },
            stopRecording = { stopVadRecording(silent = true) },
            editorInfo = ::currentEditorInfo,
        )
    }

    private val inputMethodBridge by lazy { InputMethodBridgeFactory.create(this) }

    // Keep sub-second speech durations across fragments.
    @Volatile private var speechMillisRemainder: Long = 0L

    // Wall-clock safety cap for forgotten recordings and media mistaken for speech.
    private val recordingLimit by lazy { RecordingLimitController(this, ::stopVadRecording) }

    // Keep the screen alive across brief pauses in speech.
    private val speechGraceHandler = android.os.Handler(android.os.Looper.getMainLooper())
    private val speechGraceRunnable = Runnable {
        bubbleOverlay?.setKeepScreenOnOverride(false)
    }

    private val screenOffReceiver = actionBroadcastReceiver(Intent.ACTION_SCREEN_OFF) {
        stopVadRecording(silent = true)
    }

    // Re-evaluates bubble visibility when the user unlocks the device. Without
    // this, if a keyboard was already up on the lockscreen (e.g. password
    // entry) we'd correctly hide the bubble, but on unlock — when the same IME
    // window stays focused (notification reply, app behind keyguard) — the
    // accessibility windows-changed event may not refire, leaving the bubble
    // hidden until the next focus change.
    private val userPresentReceiver = actionBroadcastReceiver(Intent.ACTION_USER_PRESENT) {
        visibilityController.update()
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onServiceConnected() {
        super.onServiceConnected()
        // Contain known framework input-method bookkeeping failures.
        AccessibilityFrameworkCrashGuard.install()
        instance = this
        AppLifecycleDiagnostics.onConnected(this)

        bubbleOverlay = BubbleOverlayController(this, bubbleCallbacks).also {
            it.create(initiallyVisible = false)
        }

        registerActionReceiver(screenOffReceiver, Intent.ACTION_SCREEN_OFF)
        registerActionReceiver(userPresentReceiver, Intent.ACTION_USER_PRESENT)

        // Initial check — if the service starts while a keyboard is already up,
        // we'd otherwise wait for the next windows-changed event.
        bubbleOverlay?.getView()?.post { visibilityController.update() }

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
        event?.let(visibilityController::onAccessibilityEvent)
    }

    private val textInserter by lazy {
        TextInserter(this, inputMethodBridge) { dictationTiming?.inserted() }
    }
    private val stylizationCoordinator by lazy { StylizationCoordinator(this) }
    private val stylizedTextDelivery by lazy {
        createStylizedTextDelivery(
            context = this,
            process = { text, mode, target ->
                dictationTiming?.process(text)
                stylizationCoordinator.process(text, mode, target).also { dictationTiming?.result(it.text.length) }
            },
            insert = ::pasteText,
            reportClientOutcome = { outcomePass, outcome, latencyBucket ->
                scope.launch(Dispatchers.IO) {
                    stylizationCoordinator.reportClientOutcome(
                        outcomePass,
                        outcome,
                        latencyBucket,
                    )
                }
            },
            showFallback = { bubbleOverlay?.showStylizationFallback() },
        )
    }

    private fun pasteText(text: String) = attemptTextInsertion(
        insert = { textInserter.insert(text) },
        reportFailure = { Log.w(TAG, "Paste: editor operation failed (${it.javaClass.simpleName})") },
    )
    private suspend fun pasteRecognizedText(
        text: String,
        uiMode: BubbleOverlayController.RecordingUiMode,
        appendVoiceMarker: Boolean = false,
    ) {
        stylizedTextDelivery.deliver(
            text,
            uiMode.stylizationInputMode(),
            appendVoiceMarker,
            visibilityController.currentForegroundPackage,
        )
    }

    private fun currentEditorInfo() = resolveCurrentEditorInfo(this, inputMethodBridge)

    @SuppressLint("MissingPermission")
    private fun startVadRecording(
        uiMode: BubbleOverlayController.RecordingUiMode =
            BubbleOverlayController.RecordingUiMode.LEGACY,
    ) {
        // Keep ownership until onDone while a stopped recorder flushes its tail.
        if (isVadActive || vadRecorder != null) return
        recordingUiMode = uiMode
        // A one-time mic permission may be revoked between service sessions.
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
        val deliverySession = TranscriptDeliverySession(
            deferred = uiMode != BubbleOverlayController.RecordingUiMode.LEGACY,
        )
        val streamingMarker = StreamingVoiceMarker(
            com.govorun.lite.util.VoiceMarkerPrefs.deliveryMarker(this),
        )
        // Open AudioRecord synchronously so a fast tap never loses the first word.
        val recorder = VadRecorder(this)
        val timing = DictationTiming().also { dictationTiming = it }
        val sessionUiMode = uiMode
        recorder.start(
            scope = scope,
            preparedCapture = pendingMicrophone.take(),
            transcriberProvider = { OfflineTranscriber.getInstance(this@LiteAccessibilityService, timing::model) },
            onRecordingDuration = timing::audio,
            minSilenceSeconds = deliverySession.minimumSilenceSeconds {
                Prefs.getPauseLengthSeconds(this)
            },
            onSegment = { text ->
                deliverySession.accept(text)?.let { releasedText ->
                    streamingMarker.accept(releasedText)?.let { chunk ->
                        pasteText(chunk)
                    }
                }
            },
            onDone = { hadText ->
                runRecordingCompletion(
                    complete = { deliverySession.deliver {
                        val contextualFragmentCount = deliverySession.fragmentCount
                        val contextualText = if (hadText) deliverySession.complete() else null
                        when {
                            sessionUiMode == BubbleOverlayController.RecordingUiMode.LEGACY -> {
                                contextualText?.let { streamingMarker.accept(it)?.let(::pasteText) }
                            }
                            contextualText != null -> {
                                AppLog.log(
                                    this@LiteAccessibilityService,
                                    "Contextual transcript: fragments=$contextualFragmentCount " +
                                        "chars=${contextualText.length}",
                                )
                                pasteRecognizedText(
                                    contextualText,
                                    sessionUiMode,
                                    appendVoiceMarker = true,
                                )
                            }
                        }
                        if (!hadText) deliverySession.discard()
                        if (sessionUiMode == BubbleOverlayController.RecordingUiMode.LEGACY) {
                            streamingMarker.flush(hadText)?.let { chunk ->
                                pasteText(chunk)
                            }
                            streamingMarker.suffix(hadText)?.let(::pasteText)
                        }
                    } },
                    restoreReady = {
                        deliverySession.connection?.close()
                        if (contextualTranscript === deliverySession) contextualTranscript = null
                        if (vadRecorder === recorder) vadRecorder = null
                        isVadActive = false
                        bubbleOverlay?.setProcessing(false)
                        bubbleOverlay?.setKeepScreenOnOverride(false)
                        visibilityController.update()
                    },
                )
            },
            // Accumulate milliseconds so short phrases are not rounded away.
            onSpeechMillis = { millis ->
                speechMillisRemainder += millis
                val whole = speechMillisRemainder / 1000L
                if (whole > 0L) {
                    AndroidStatsRecorder.create(this@LiteAccessibilityService).addSeconds(whole)
                    speechMillisRemainder -= whole * 1000L
                }
            },
            onRecognitionMetric = { speechMillis, processingMillis, success ->
                timing.recognized()
                AndroidStatsRecorder.create(this@LiteAccessibilityService).recordRecognitionPerformance(
                    speechMillis,
                    processingMillis,
                    success,
                )
            },
            onRecognitionStarted = timing::recognize,
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
        )
        if (!recorder.isActive) {
            deliverySession.discard()
            bubbleOverlay?.setRecording(false, uiMode)
            bubbleOverlay?.setProcessing(false)
            bubbleOverlay?.setKeepScreenOnOverride(false)
            Haptics.reject(this)
            AppLog.log(this, "Service: VAD start rejected; overlay restored to idle")
            return
        }
        vadRecorder = recorder
        isVadActive = true
        if (uiMode != BubbleOverlayController.RecordingUiMode.LEGACY) deliverySession.connection =
            com.govorun.lite.stylization.PreparedStylesConnection(scope)
        // Optional account preparation never precedes microphone acquisition.
        scope.launch(Dispatchers.IO) {
            AppLog.log(this@LiteAccessibilityService, "Service: VAD start (mode=$uiMode)")
            if (uiMode != BubbleOverlayController.RecordingUiMode.LEGACY &&
                officialStylizationActive(this@LiteAccessibilityService)) {
                GovorunAccountRepository.prepareOfficialStylePassesForUpcomingDictation()
            }
        }
        if (uiMode != BubbleOverlayController.RecordingUiMode.LEGACY) {
            contextualTranscript = deliverySession
        }
        Haptics.longPress(this)
        bubbleOverlay?.setRecording(true, uiMode)
        scheduleAutoStop()
        Log.i(TAG, "VAD recording started")
    }

    private fun stopVadRecording(silent: Boolean = false) {
        if (!isVadActive) return
        dictationTiming?.release()
        isVadActive = false
        // Speech time is accounted by VAD callbacks. Graceful stop flushes
        // the buffered tail so a quick confirmation does not lose the last word.
        vadRecorder?.stop()
        recordingLimit.cancel()
        speechGraceHandler.removeCallbacks(speechGraceRunnable)
        if (!silent) Haptics.doubleTap(this)
        if (recordingUiMode != BubbleOverlayController.RecordingUiMode.LEGACY) {
            // Keep the processing indicator alive while the recorded tail finishes.
            bubbleOverlay?.finishRecordingAndStartProcessing()
        } else {
            bubbleOverlay?.setRecording(false)
            bubbleOverlay?.setKeepScreenOnOverride(false)
        }
        Log.i(TAG, "VAD recording stopped")
    }

    /** Discard the current contextual recording without transcription,
     *  stylization, insertion, or statistics side effects. */
    private fun cancelVadRecording() {
        if (!isVadActive) return
        isVadActive = false
        contextualTranscript?.discard()
        vadRecorder?.cancel()
        vadRecorder = null
        recordingLimit.cancel()
        speechGraceHandler.removeCallbacks(speechGraceRunnable)
        bubbleOverlay?.setRecording(false, recordingUiMode)
        bubbleOverlay?.setProcessing(false)
        bubbleOverlay?.setKeepScreenOnOverride(false)
        Haptics.reject(this)
        AppLog.log(this, "Service: contextual recording cancelled")
        visibilityController.update()
    }

    /** Supplies native preferences to the common recording plan. */
    private fun scheduleAutoStop() {
        recordingLimit.arm(
            officialReady = officialStylizationActive(this),
            streaming = recordingUiMode == BubbleOverlayController.RecordingUiMode.LEGACY,
            selfHosted = com.govorun.lite.stylization.StylizationServerPreferences.mode(this) ==
                com.govorun.lite.stylization.StylizationServerMode.SELF_HOSTED,
            keepScreenOn = Prefs.isKeepScreenOnEnabled(this),
            fallbackMinutes = Prefs.getAutoStopMinutes(this),
        )
    }

    override fun onInterrupt() = AppLifecycleDiagnostics.onInterrupted(this)

    /** Commit the classified gesture; only then allow recognition and delivery. */
    private val bubbleCallbacks = bubbleRecordingCallbacks(
        onTap = {
            if (isVadActive) {
                stopVadRecording()
            } else if (
                RecordingModePrefs.read(this@LiteAccessibilityService) ==
                RecordingModePrefs.REPEAT_TAP
            ) {
                startVadRecording(uiMode = BubbleOverlayController.RecordingUiMode.CONFIRM)
            } else {
                startVadRecording()
            }
        },
        onHoldStart = {
            startVadRecording(uiMode = BubbleOverlayController.RecordingUiMode.HOLD)
        },
        onHoldStop = { silent ->
            if (silent) cancelVadRecording() else stopVadRecording()
        },
        onConfirm = {
            if (recordingUiMode == BubbleOverlayController.RecordingUiMode.CONFIRM) {
                stopVadRecording()
            }
        },
        onCancel = ::cancelVadRecording,
        isVadActive = { isVadActive },
        onPointerDown = { if (!isVadActive && vadRecorder == null) pendingMicrophone.prepare() },
        onPointerCancel = { pendingMicrophone.discard(scope) },
    )

    override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
        super.onConfigurationChanged(newConfig)
        // Wallpaper colour changes arrive here on API 31+. Rebuild the bubble
        // with a fresh DC-wrapped context so the new accent takes effect
        // without the user having to toggle the service.
        bubbleOverlay?.rebuild(initiallyVisible = visibilityController.isImeVisible)
    }

    /**
     * Called from Settings after the user moves the transparency slider — the
     * existing BubbleView picks up the new alpha immediately; no rebuild.
     */
    fun applyBubbleAlphaFromPrefs() {
        bubbleOverlay?.applyAlpha()
    }

    internal fun showStylizationFallbackForTest() {
        bubbleOverlay?.showStylizationFallback()
        Haptics.reject(this)
    }

    /**
     * Called from Settings after the user moves the size slider. Size changes
     * affect onMeasure() output, so we rebuild the bubble (same path as the
     * wallpaper-colour reaction) instead of trying to re-layout in place.
     */
    fun applyBubbleSizeFromPrefs() {
        bubbleOverlay?.applySize(initiallyVisible = visibilityController.isImeVisible)
    }

    /**
     * True when the bubble is currently painted on screen.
     */
    fun isBubbleVisible(): Boolean = bubbleOverlay?.isVisible() == true

    fun isAlwaysShowEnabled(): Boolean = visibilityController.isAlwaysShowEnabled()

    fun toggleAlwaysShow() = visibilityController.toggleAlwaysShow()

    /** Quick Settings Tile read-out for the silence tile. */
    fun isSilenceEnabled(): Boolean = visibilityController.isSilenceEnabled()

    fun toggleSilence() = visibilityController.toggleSilence()

    fun applyBubbleSideFromPrefs() {
        bubbleOverlay?.applySide()
    }

    fun applyBubbleEdgeMarginFromPrefs() {
        bubbleOverlay?.applyEdgeMargin()
    }

    fun applyBubbleThemeFromPrefs() =
        bubbleOverlay?.rebuild(initiallyVisible = visibilityController.isImeVisible)

    override fun onDestroy() {
        pendingMicrophone.discard(scope)
        instance = null
        AppLifecycleDiagnostics.onStopped(this)
        // Service destruction must never flush or insert a half-finished
        // phrase. It also has to release the process-wide recognition lease
        // before another Binder client can start.
        isVadActive = false
        contextualTranscript?.discard()
        vadRecorder?.cancel()
        vadRecorder = null
        recordingLimit.cancel()
        scope.cancel()
        try { unregisterReceiver(screenOffReceiver) } catch (_: Exception) {}
        try { unregisterReceiver(userPresentReceiver) } catch (_: Exception) {}
        visibilityController.destroy()
        bubbleOverlay?.destroy()
        bubbleOverlay = null; super.onDestroy()
    }
}
