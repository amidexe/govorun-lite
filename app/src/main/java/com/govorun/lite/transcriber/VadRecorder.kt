package com.govorun.lite.transcriber

import android.annotation.SuppressLint
import android.content.Context
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.SystemClock
import android.util.Log
import com.govorun.lite.util.Prefs
import com.k2fsa.sherpa.onnx.SileroVadModelConfig
import com.k2fsa.sherpa.onnx.TenVadModelConfig
import com.k2fsa.sherpa.onnx.Vad
import com.k2fsa.sherpa.onnx.VadModelConfig
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * VAD-based recorder using producer-consumer architecture:
 *  - Reader coroutine reads mic → Silero VAD → pushes detected speech segments into a Channel.
 *  - Transcriber coroutine pops segments and transcribes independently.
 *
 * Silero VAD model is bundled in assets/ (629 KB) and copied to files dir on first run.
 */
class VadRecorder(private val context: Context) {

    companion object {
        private const val TAG = "VadRecorder"
        private const val SAMPLE_RATE = 16000
        private const val VAD_WINDOW_SIZE = 512
        private const val VAD_MODEL_ASSET = "silero_vad.onnx"
        private const val SEGMENT_QUEUE_CAPACITY = 8
        // Tail buffer safety net for manual stop before VAD's silence threshold.
        // 0.25s minimum matches minSpeechDuration; 60s cap matches maxSpeechDuration.
        private const val MIN_TAIL_BYTES = SAMPLE_RATE * 2 / 4           //  0.25s
        private const val MAX_TAIL_BYTES = SAMPLE_RATE * 2 * 60          // 60s
        // After the user taps stop we keep the mic running for a short linger so
        // the tail end of a word they spoke while tapping still reaches VAD
        // instead of being sliced off. Tuned at 250ms: long enough for the
        // trailing phoneme of a single word (typically 150-200ms), short enough
        // that a whole following word can't land inside the window.
        private const val LINGER_AFTER_STOP_MS = 250L

        /**
         * ONNX-backed Silero VAD is expensive to construct (100-500ms depending
         * on CPU). We cache one instance for the lifetime of the process and
         * reset() its internal state between sessions, so the user's tap → mic-on
         * latency stays tight and their first spoken word isn't lost.
         */
        @Volatile private var sharedVad: Vad? = null
        @Volatile private var sharedMinSilenceSeconds: Float = 0f
        private val vadLock = Any()

        fun warmUp(context: Context) {
            getOrBuildVad(context, Prefs.getPauseLengthSeconds(context))
        }

        // Returns the cached Vad, rebuilding it if the user's pause-length
        // pref changed since the last build. Silero's minSilenceDuration is
        // baked into the model config at construction time, so changing it
        // requires throwing away the instance.
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
                maxSpeechDuration = 30.0f
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
    private var job: kotlinx.coroutines.Job? = null
    @Volatile private var audioRecord: AudioRecord? = null
    @Volatile private var stopAtMs: Long = Long.MAX_VALUE

    @SuppressLint("MissingPermission")
    fun start(
        scope: CoroutineScope,
        transcriberProvider: suspend () -> Transcriber,
        onSegment: suspend (String) -> Unit,
        // Called once per VAD-detected speech segment with that segment's
        // duration in MILLISECONDS. Caller is responsible for accumulating
        // and rolling over to seconds (a 700ms «ага» would otherwise round
        // to 0 and disappear from the counter). Used to feed StatsStore —
        // pure speech time, not session time.
        onSpeechMillis: (Long) -> Unit = {},
        // Edge callback for «speech currently audible» — fires only on
        // transitions (silence→speech and speech→silence), not on every
        // VAD window. Used by the service to keep the screen alive while
        // the user is actively speaking, even when the user has NOT opted
        // into the persistent keep-screen-on flag. Without this hook,
        // ongoing dictation is killed by Android's screen timeout because
        // speech is not a touch event and we don't poke userActivity.
        onSpeechActive: (Boolean) -> Unit = {},
        // useVad=true (default): Silero splits audio into segments on
        // pauses, each segment transcribed and pasted independently —
        // the right behaviour for tap-toggle dictation where the user
        // expects paragraph breaks where they paused.
        // useVad=false: pure raw-PCM accumulator, no segmentation.
        // Whole hold-duration audio is sent to GigaAM in one shot,
        // so a phrase like "сегодня в 25 минут 6" stays as one
        // contextual recognition pass instead of being split into
        // independent fragments and concatenated.
        useVad: Boolean = true,
    ) {
        if (isActive) return
        isActive = true
        stopAtMs = Long.MAX_VALUE

        // Create the AudioRecord + start capturing *synchronously* on the caller
        // (Main) thread, before any coroutine dispatching — every millisecond
        // between tap and mic-on is a millisecond of the user's first word that
        // will never be heard. On a modern device this is ~5-20 ms.
        val windowBytes = VAD_WINDOW_SIZE * 2
        val bufferSize = AudioRecord.getMinBufferSize(
            SAMPLE_RATE, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT
        ).coerceAtLeast(windowBytes * 8)
        val audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC, SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, bufferSize
        )
        // The AudioRecord constructor never throws — it just leaves the
        // object in STATE_UNINITIALIZED if mic acquisition failed (mic
        // busy with another app, permission revoked at runtime, or the
        // previous recorder hasn't fully released its handle yet). Calling
        // startRecording() on an uninitialized AudioRecord throws, which
        // crashes the whole accessibility service. Bail gracefully here.
        if (audioRecord.state != AudioRecord.STATE_INITIALIZED) {
            Log.w(TAG, "AudioRecord uninitialized — mic busy / permission lost; aborting start")
            audioRecord.release()
            isActive = false
            return
        }
        this.audioRecord = audioRecord
        audioRecord.startRecording()
        Log.i(TAG, "VAD recording started (mic capturing)")

        job = scope.launch(Dispatchers.IO) {
            val vad = getOrBuildVad(context, Prefs.getPauseLengthSeconds(context))
            // Cached instance carries state from previous session — wipe it.
            // ALL Vad accesses below are wrapped in synchronized(vadLock):
            // the underlying sherpa-onnx Vad is not thread-safe, and rapid
            // tap-toggle creates overlapping VadRecorder sessions whose
            // readers race on this same shared instance. Without the lock
            // the second reader hits "Vad_acceptWaveform: NULL input
            // supplied" — a SIGSEGV in native code that takes down the
            // whole accessibility service process, and Android then refuses
            // to restart it after a few crashes in a row. Same lock as
            // getOrBuildVad/release so a concurrent rebuild is safe too.
            try { synchronized(vadLock) { vad.reset() } } catch (e: Exception) { Log.w(TAG, "vad.reset failed: ${e.message}") }

            val segmentChannel = Channel<ByteArray>(capacity = SEGMENT_QUEUE_CAPACITY)

            val readerJob = launch {
                if (!useVad) {
                    // Raw-PCM accumulator path — used by hold-to-talk so the
                    // entire utterance reaches GigaAM as one block, with the
                    // model's full context window applied to the whole phrase.
                    val pcmWindow = ByteArray(windowBytes)
                    val rawBuffer = ByteArrayOutputStream()
                    try {
                        while (coroutineContext.isActive && SystemClock.elapsedRealtime() < stopAtMs) {
                            val read = audioRecord.read(pcmWindow, 0, windowBytes)
                            if (read > 0) rawBuffer.write(pcmWindow, 0, read)
                            else if (read < 0) break
                        }
                    } finally {
                        if (rawBuffer.size() >= MIN_TAIL_BYTES) {
                            val pcm = rawBuffer.toByteArray()
                            Log.i(TAG, "Raw mode: queueing ${pcm.size}B (~${pcm.size / 32}ms) as single segment")
                            if (!segmentChannel.trySend(pcm).isSuccess) {
                                Log.w(TAG, "Raw segment queue full — dropped")
                            }
                            // Hold-mode: count the whole captured PCM as speech.
                            // The user was actively holding their finger down, so
                            // by definition they intended to be speaking the whole
                            // time — no spurious silence to filter out.
                            // bytes/2 = shorts (16-bit mono); * 1000 / SAMPLE_RATE = ms.
                            val millis = pcm.size.toLong() / 2 * 1000L / SAMPLE_RATE
                            if (millis > 0L) onSpeechMillis(millis)
                        } else {
                            Log.w(TAG, "Raw mode: tail too short (${rawBuffer.size()}B) — nothing to transcribe")
                        }
                        rawBuffer.reset()
                        segmentChannel.close()
                        Log.i(TAG, "Raw reader stopped")
                    }
                    return@launch
                }

                val pcmWindow = ByteArray(windowBytes)
                // Raw PCM captured since the last VAD segment boundary. Acts as a
                // safety-net if the user stops recording mid-utterance and VAD's
                // internal buffer has already been partially drained by flush().
                val tailBuffer = ByteArrayOutputStream()
                var segmentsEmitted = 0
                // Tracks isSpeechDetected() across windows so we only emit
                // edge events to onSpeechActive (start of speech, end of
                // speech). Initial false matches VAD's post-reset state.
                var prevSpeechDetected = false

                fun drainVadQueue() {
                    while (!vad.empty()) {
                        val segment = vad.front()
                        vad.pop()

                        val segPcm = ByteArray(segment.samples.size * 2)
                        val bb = ByteBuffer.wrap(segPcm).order(ByteOrder.LITTLE_ENDIAN)
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
                    while (coroutineContext.isActive && SystemClock.elapsedRealtime() < stopAtMs) {
                        var offset = 0
                        while (offset < windowBytes &&
                               coroutineContext.isActive &&
                               SystemClock.elapsedRealtime() < stopAtMs) {
                            val read = audioRecord.read(pcmWindow, offset, windowBytes - offset)
                            if (read > 0) offset += read
                            else if (read < 0) break
                        }
                        if (offset < windowBytes) {
                            // Partial read (AudioRecord stopped mid-window). Save
                            // what we got to the tail buffer so it still reaches
                            // the fallback flush below; it's too short to feed to
                            // VAD (which demands exactly VAD_WINDOW_SIZE samples).
                            if (offset > 0) tailBuffer.write(pcmWindow, 0, offset)
                            break
                        }

                        if (tailBuffer.size() + windowBytes > MAX_TAIL_BYTES) {
                            // Cap memory usage — drop oldest half of tail. In practice
                            // VAD will cut before this on any natural speech.
                            val trimmed = tailBuffer.toByteArray().copyOfRange(
                                tailBuffer.size() / 2, tailBuffer.size()
                            )
                            tailBuffer.reset()
                            tailBuffer.write(trimmed)
                        }
                        tailBuffer.write(pcmWindow, 0, windowBytes)

                        val shortBuf = ByteBuffer.wrap(pcmWindow).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
                        val samples = FloatArray(VAD_WINDOW_SIZE) { shortBuf.get().toFloat() / 32768f }
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
                    }
                } finally {
                    if (prevSpeechDetected) {
                        // Reader is shutting down while VAD still considered
                        // the audio "speech". Emit a final false so the
                        // service can drop any keep-screen-on override and
                        // the device returns to normal sleep behaviour.
                        prevSpeechDetected = false
                        withContext(Dispatchers.Main) { onSpeechActive(false) }
                    }
                    // Flush any speech still inside VAD's buffer (user stopped before
                    // min_silence elapsed) so the last utterance isn't lost.
                    val beforeFlush = segmentsEmitted
                    try {
                        synchronized(vadLock) {
                            vad.flush()
                            drainVadQueue()
                        }
                    } catch (e: Exception) {
                        Log.w(TAG, "VAD flush failed: ${e.message}")
                    }
                    // If flush didn't surface a segment and we captured enough audio
                    // since the last boundary, send the raw tail as a fallback so the
                    // user's last phrase always lands in the pipeline.
                    val tailMs = tailBuffer.size() / 32
                    if (segmentsEmitted == beforeFlush && tailBuffer.size() >= MIN_TAIL_BYTES) {
                        Log.i(TAG, "VAD produced no tail segment — sending raw PCM fallback (${tailBuffer.size()} bytes / ~${tailMs}ms)")
                        val fallback = tailBuffer.toByteArray()
                        if (!segmentChannel.trySend(fallback).isSuccess) {
                            Log.w(TAG, "Fallback segment could not be queued — dropped")
                        }
                    } else if (segmentsEmitted == beforeFlush) {
                        Log.w(TAG, "Tail below MIN threshold (${tailBuffer.size()} bytes / ~${tailMs}ms) — nothing to transcribe")
                    }
                    tailBuffer.reset()
                    segmentChannel.close()
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
                readerJob.cancelAndJoin()
                try { audioRecord.stop() } catch (_: Exception) {}
                audioRecord.release()
                this@VadRecorder.audioRecord = null
                return@launch
            }

            try {
                for (segPcm in segmentChannel) {
                    val segMs = segPcm.size / 32
                    try {
                        val t0 = SystemClock.elapsedRealtime()
                        transcriber.startAudio()
                        transcriber.sendAudioChunk(segPcm)
                        val text = transcriber.stopAudioAndGetTranscript()
                        val dt = SystemClock.elapsedRealtime() - t0
                        // Privacy: never write the recognised text to logcat
                        // (visible to anyone with adb access). See OfflineTranscriber
                        // for the same reasoning. Length only.
                        Log.i(TAG, "Transcribed ${segPcm.size}B (~${segMs}ms audio) in ${dt}ms → ${text.length} chars")
                        if (text.isNotBlank()) {
                            withContext(Dispatchers.Main) { onSegment(text) }
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Segment transcription failed (${segMs}ms audio): ${e.message}")
                    }
                }
            } finally {
                readerJob.cancelAndJoin()
                try { audioRecord.stop() } catch (_: Exception) {}
                audioRecord.release()
                this@VadRecorder.audioRecord = null
                this@VadRecorder.isActive = false
                stopAtMs = Long.MAX_VALUE
                // Keep sharedVad alive — it's reused across sessions.
                Log.i(TAG, "VAD recording stopped")
            }
        }
    }

    /**
     * Graceful stop: arms a short linger window so trailing audio the user spoke
     * while tapping the bubble still reaches VAD, then lets the reader exit on
     * its own. The reader's finally block flushes VAD and the raw PCM tail into
     * the consumer, which keeps running in the background to transcribe and
     * paste. Deliberately does NOT touch audioRecord — the reader owns it.
     */
    fun stop() {
        if (!isActive) return
        stopAtMs = SystemClock.elapsedRealtime() + LINGER_AFTER_STOP_MS
        Log.i(TAG, "Stop requested — linger ${LINGER_AFTER_STOP_MS}ms then exit")
    }
}
