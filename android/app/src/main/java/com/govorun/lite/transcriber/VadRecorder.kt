package com.govorun.lite.transcriber

import android.content.Context
import android.os.SystemClock
import android.util.Log
import com.govorun.lite.util.Prefs
import com.k2fsa.sherpa.onnx.SileroVadModelConfig
import com.k2fsa.sherpa.onnx.TenVadModelConfig
import com.k2fsa.sherpa.onnx.Vad
import com.k2fsa.sherpa.onnx.VadModelConfig
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

/** Buffered microphone → shared VAD → ordered recognition. */
class VadRecorder(private val context: Context) {

    companion object {
        private const val TAG = "VadRecorder"
        internal const val SAMPLE_RATE = 16000
        internal const val VAD_WINDOW_SIZE = 512
        private const val VAD_MODEL_ASSET = "silero_vad.onnx"
        private const val SEGMENT_QUEUE_CAPACITY = 8
        internal const val MAX_SPEECH_SEGMENT_SECONDS = 30.0f
        // Tail buffer is only an emergency flush path. It may retain two
        // maximum segments so trimming cannot race a 30-second VAD boundary.
        private const val MIN_TAIL_BYTES = SAMPLE_RATE * 2 / 4           //  0.25s
        private const val MAX_TAIL_BYTES = SAMPLE_RATE * 2 * 60          // 60s

        // Reuse VAD between sessions; microphone capture never waits for it.
        @Volatile private var sharedVad: Vad? = null
        @Volatile private var sharedMinSilenceSeconds: Float = 0f
        private val vadLock = Any()

        fun warmUp(context: Context) {
            getOrBuildVad(context, Prefs.getPauseLengthSeconds(context))
        }

        internal fun createSessionVad(context: Context, minSilenceSeconds: Float): Vad =
            synchronized(vadLock) { buildVad(context, minSilenceSeconds) }

        // Pause changes require a new native VAD.
        fun getOrBuildVad(context: Context, minSilenceSeconds: Float): Vad =
            synchronized(vadLock) {
                val existing = sharedVad
                if (existing != null && sharedMinSilenceSeconds == minSilenceSeconds) {
                    existing
                } else {
                    existing?.release()
                    val fresh = buildVad(context, minSilenceSeconds)
                    sharedVad = fresh
                    sharedMinSilenceSeconds = minSilenceSeconds
                    fresh
                }
            }

        private fun buildVad(context: Context, minSilenceSeconds: Float): Vad {
            val modelPath = ensureVadModelPath(context)
            val sileroConfig = SileroVadModelConfig(
                model = modelPath,
                threshold = 0.5f,
                minSilenceDuration = minSilenceSeconds,
                minSpeechDuration = 0.25f,
                windowSize = VAD_WINDOW_SIZE,
                maxSpeechDuration = MAX_SPEECH_SEGMENT_SECONDS
            )
            val vadConfig = VadModelConfig(
                sileroVadModelConfig = sileroConfig,
                tenVadModelConfig = TenVadModelConfig(),
                sampleRate = SAMPLE_RATE,
                numThreads = 1,
                provider = "cpu",
                debug = false
            )
            return Vad(null, vadConfig)
        }

        private fun ensureVadModelPath(context: Context): String {
            val dir = File(context.filesDir, "models/vad").also { it.mkdirs() }
            val f = File(dir, VAD_MODEL_ASSET)
            if (f.exists() && f.length() > 100_000) return f.absolutePath
            Log.i(TAG, "Extracting bundled Silero VAD model from assets…")
            context.assets.open(VAD_MODEL_ASSET).use { input ->
                f.outputStream().use { output -> input.copyTo(output) }
            }
            Log.i(TAG, "VAD model ready (${f.length()} bytes)")
            return f.absolutePath
        }
    }

    @Volatile var isActive = false
        private set
    private var recognitionLease: RecognitionSessionArbiter.Lease? = null
    private var job: kotlinx.coroutines.Job? = null
    @Volatile private var microphone: MicrophoneCapture? = null
    private var stopTailMilliseconds = 0L
    @Volatile private var stopAtMs: Long = Long.MAX_VALUE
    // Explicit cancel differs from stop: stop flushes and delivers the tail,
    // cancel releases the microphone but discards every queued or in-flight
    // result. This is required by the tap-to-confirm overlay's cross button.
    @Volatile private var discardOutput = false

    fun start(
        scope: CoroutineScope,
        transcriberProvider: suspend () -> Transcriber,
        onSegment: suspend (String) -> Unit,
        // Always called after recognition, including empty recordings.
        onDone: suspend (hadText: Boolean) -> Unit = {},
        // Accumulate milliseconds so short segments are not rounded away.
        onSpeechMillis: (Long) -> Unit = {},
        // Daily performance buckets contain no audio or text.
        onRecognitionMetric: (speechMillis: Long, processingMillis: Long, success: Boolean) -> Unit =
            { _, _, _ -> },
        onRecognitionStarted: () -> Unit = {},
        onRecordingDuration: (Long) -> Unit = {},
        // Notify only on speech/silence transitions.
        onSpeechActive: (Boolean) -> Unit = {},
        // Dictionary capture can override the normal pause threshold.
        minSilenceSeconds: Float = Prefs.getPauseLengthSeconds(context),
        preparedCapture: MicrophoneCapture? = null,
    ) {
        if (isActive) { preparedCapture?.discard(scope); return }
        val lease = RecognitionSessionCoordinator.process.acquirePrimary() ?: run { preparedCapture?.discard(scope); return }
        recognitionLease = lease
        isActive = true
        stopAtMs = Long.MAX_VALUE
        discardOutput = false

        val input = preparedCapture ?: MicrophoneCapture.open(context, scope)
        if (input == null) { isActive = false; releaseRecognitionLease(); return }
        microphone = input
        val windowBytes = VAD_WINDOW_SIZE * 2
        stopTailMilliseconds = com.govorun.lite.shared.SharedDictationRules.processingLimits
            .getLong("stopTailMilliseconds")
        Log.i(TAG, "VAD recording started (mic capturing)")

        job = scope.launch(Dispatchers.IO) {
            val capture = input.capture
            val vad = try {
                getOrBuildVad(context, minSilenceSeconds)
            } catch (error: Exception) {
                Log.e(TAG, "VAD initialization failed: ${error.message}")
                onRecognitionMetric(0, 0, false)
                releaseRecognitionLease()
                input.close()
                this@VadRecorder.microphone = null
                withContext(NonCancellable + Dispatchers.Main) { onDone(false) }
                this@VadRecorder.isActive = false
                stopAtMs = Long.MAX_VALUE
                return@launch
            }
            // All access to the shared native VAD uses the same lock.
            try { synchronized(vadLock) { vad.reset() } } catch (e: Exception) { Log.w(TAG, "vad.reset failed: ${e.message}") }

            val segmentChannel = Channel<ByteArray>(capacity = SEGMENT_QUEUE_CAPACITY)

            val readerJob = launch {
                // Raw PCM captured since the last VAD segment boundary. Acts as a
                // safety-net if the user stops recording mid-utterance and VAD's
                // internal buffer has already been partially drained by flush().
                val tailBuffer = ByteArrayOutputStream()
                var segmentsEmitted = 0
                var capturedBytes = 0L
                // Tracks isSpeechDetected() across windows so we only emit
                // edge events to onSpeechActive (start of speech, end of
                // speech). Initial false matches VAD's post-reset state.
                var prevSpeechDetected = false

                fun drainVadQueue() {
                    while (!vad.empty()) {
                        val segment = vad.front()
                        vad.pop()

                        // Speech may begin with the very first captured frame. Keep
                        // the short opening that VAD omits while detecting speech;
                        // long leading silence still stays outside recognition.
                        val openingBytes = if (segmentsEmitted == 0 &&
                            segment.start in 1..(SAMPLE_RATE / 4)) segment.start * 2 else 0
                        val segPcm = ByteArray(segment.samples.size * 2 + openingBytes)
                        if (openingBytes > 0) {
                            tailBuffer.toByteArray().copyInto(segPcm, 0, 0, openingBytes)
                        }
                        val bb = ByteBuffer.wrap(segPcm).order(ByteOrder.LITTLE_ENDIAN)
                        bb.position(openingBytes)
                        for (s in segment.samples) {
                            bb.putShort((s * 32767f).toInt().coerceIn(-32768, 32767).toShort())
                        }

                        if (!segmentChannel.trySend(segPcm).isSuccess) {
                            Log.w(TAG, "Segment queue full — dropping segment (transcriber too slow)")
                        }
                        // A VAD segment = silence confirmed, so whatever we captured up
                        // to now is safely in the transcriber pipeline. Drop the tail.
                        tailBuffer.reset()
                        segmentsEmitted++

                        // Report this segment's duration in milliseconds. Short
                        // phrases (~700ms «ага») would round to 0 seconds and
                        // disappear if we converted here — accumulator on the
                        // caller side preserves them.
                        val millis = segment.samples.size * 1000L / SAMPLE_RATE
                        if (millis > 0L) onSpeechMillis(millis)
                    }
                }

                try {
                    for (pcmWindow in capture.frames) {
                        capturedBytes += pcmWindow.size
                        if (tailBuffer.size() + pcmWindow.size > MAX_TAIL_BYTES) {
                            // Cap memory usage — drop oldest half of tail. In practice
                            // VAD will cut before this on any natural speech.
                            val trimmed = tailBuffer.toByteArray().copyOfRange(
                                tailBuffer.size() / 2, tailBuffer.size()
                            )
                            tailBuffer.reset()
                            tailBuffer.write(trimmed)
                        }
                        tailBuffer.write(pcmWindow)

                        val shortBuf = ByteBuffer.wrap(pcmWindow).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
                        // An interrupted microphone read may return a partial
                        // window. Feed its sound too, padding only the missing end.
                        val samples = FloatArray(VAD_WINDOW_SIZE) {
                            if (shortBuf.hasRemaining()) shortBuf.get().toFloat() / 32768f else 0f
                        }
                        // acceptWaveform + drain MUST be in the same critical
                        // section: drain reads vad.empty/front/pop, and another
                        // reader could mutate Vad state between them, leaving
                        // us reading partially-popped segments.
                        val speechNow = synchronized(vadLock) {
                            vad.acceptWaveform(samples)
                            drainVadQueue()
                            vad.isSpeechDetected()
                        }
                        if (speechNow != prevSpeechDetected) {
                            prevSpeechDetected = speechNow
                            withContext(Dispatchers.Main) { onSpeechActive(speechNow) }
                        }
                        if (pcmWindow.size < windowBytes) break
                    }
                } finally {
                    onRecordingDuration(capturedBytes * 1000L / (SAMPLE_RATE * 2))
                    val stoppedByButton = stopAtMs != Long.MAX_VALUE
                    val beforeFlush = segmentsEmitted
                    if (!stoppedByButton) {
                        try {
                            synchronized(vadLock) { vad.flush(); drainVadQueue() }
                        } catch (e: Exception) { Log.w(TAG, "VAD flush failed: ${e.message}") }
                    }
                    // Button release ends the phrase: send every captured sample
                    // since the last completed segment, without another VAD decision.
                    if ((stoppedByButton || segmentsEmitted == beforeFlush) &&
                        !discardOutput && tailBuffer.size() >= if (stoppedByButton) 2 else MIN_TAIL_BYTES) {
                        val tail = tailBuffer.toByteArray()
                        if (!segmentChannel.trySend(tail).isSuccess) {
                            Log.w(TAG, "Tail segment could not be queued")
                        }
                        onSpeechMillis(tail.size * 1000L / (SAMPLE_RATE * 2))
                    }
                    tailBuffer.reset()
                    segmentChannel.close()
                    if (prevSpeechDetected) {
                        withContext(Dispatchers.Main) { onSpeechActive(false) }
                    }
                    Log.i(TAG, "VAD reader stopped ($segmentsEmitted segments emitted this session)")
                }
            }

            // Fetch the transcriber lazily — AudioRecord is already capturing,
            // so this happens in parallel with the user's first words reaching
            // the segment channel. First call can burn 100-500 ms on model load.
            val transcriber = try {
                transcriberProvider()
            } catch (e: Exception) {
                Log.e(TAG, "Transcriber fetch failed: ${e.message}")
                onRecognitionMetric(0, 0, false)
                // Release the process-wide engine slot before any suspending
                // cleanup. If the parent scope is already cancelled,
                // cancelAndJoin() throws immediately and code after it would
                // otherwise never run.
                releaseRecognitionLease()
                readerJob.cancel()
                withContext(NonCancellable) {
                    input.close()
                    readerJob.join()
                    this@VadRecorder.microphone = null
                    withContext(Dispatchers.Main) { onDone(false) }
                    this@VadRecorder.isActive = false
                }
                return@launch
            }

            var textInserted = 0
            try {
                for (segPcm in segmentChannel) {
                    val segMs = segPcm.size / 32
                    try {
                        val t0 = SystemClock.elapsedRealtime()
                        onRecognitionStarted()
                        val text = transcriber.transcribe(segPcm)
                        val dt = SystemClock.elapsedRealtime() - t0
                        onRecognitionMetric(segMs.toLong(), dt, true)
                        // Privacy: never write the recognised text to logcat
                        // (visible to anyone with adb access). See OfflineTranscriber
                        // for the same reasoning. Length only.
                        Log.i(TAG, "Transcribed ${segPcm.size}B (~${segMs}ms audio) in ${dt}ms → ${text.length} chars")
                        if (!discardOutput && text.isNotBlank()) {
                            withContext(Dispatchers.Main) { onSegment(text) }
                            textInserted++
                        }
                    } catch (e: Exception) {
                        onRecognitionMetric(segMs.toLong(), 0, false)
                        Log.e(TAG, "Segment transcription failed (${segMs}ms audio): ${e.message}")
                    }
                }
            } finally {
                // Release the lease before suspending, even on service cancellation.
                releaseRecognitionLease()
                readerJob.cancel()
                withContext(NonCancellable) {
                    input.close()
                    readerJob.join()
                    this@VadRecorder.microphone = null
                    try {
                        withContext(Dispatchers.Main) {
                            onDone(!discardOutput && textInserted > 0)
                        }
                    } finally {
                        this@VadRecorder.isActive = false
                        stopAtMs = Long.MAX_VALUE
                    }
                }
                // Keep sharedVad alive — it's reused across sessions.
                Log.i(TAG, "VAD recording stopped")
            }
        }
        job?.invokeOnCompletion {
            if (microphone === input && isActive) {
                releaseRecognitionLease()
                input.discard(scope)
                microphone = null
                isActive = false
            }
        }
    }

    /** Button release ends capture; already saved PCM goes straight to recognition. */
    fun stop() {
        if (!isActive || stopAtMs != Long.MAX_VALUE) return
        stopAtMs = SystemClock.elapsedRealtime() + stopTailMilliseconds
        microphone?.stop()
        Log.i(TAG, "Stop requested — delivering captured audio")
    }

    /** Abort without recognition delivery; cleanup also releases the lease. */
    fun cancel() {
        if (!isActive) return
        discardOutput = true
        stopAtMs = SystemClock.elapsedRealtime()
        // Stop blocking microphone reads before cancelling recognition.
        microphone?.stop()
        job?.cancel()
        Log.i(TAG, "Cancel requested — captured audio and results will be discarded")
    }

    private fun releaseRecognitionLease() {
        RecognitionSessionCoordinator.process.release(recognitionLease)
        recognitionLease = null
    }
}
