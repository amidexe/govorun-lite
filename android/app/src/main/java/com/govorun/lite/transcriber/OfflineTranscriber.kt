package com.govorun.lite.transcriber

import android.content.Context
import android.util.Log
import com.govorun.lite.model.GigaAmModel
import com.k2fsa.sherpa.onnx.FeatureConfig
import com.k2fsa.sherpa.onnx.OfflineModelConfig
import com.k2fsa.sherpa.onnx.OfflineRecognizer
import com.k2fsa.sherpa.onnx.OfflineRecognizerConfig
import com.k2fsa.sherpa.onnx.OfflineTransducerModelConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Offline ASR using sherpa-onnx with GigaAM v3 E2E RNNT (NeMo transducer).
 * Singleton — keeps the CPU recognizer in memory across recordings.
 */
class OfflineTranscriber private constructor(
    private val recognizer: OfflineRecognizer,
    /** Which ONNX execution provider actually loaded the model: "nnapi" or "cpu". */
    val activeProvider: String
) : Transcriber {

    // Guards every call into the native sherpa-onnx session — both the hot
    // path (createStream/decode/getResult) and release(). Without this, a
    // release() on the IME-hidden path could race with an in-flight
    // transcription tail and SEGV inside libsherpa-onnx-jni.so when
    // createStream() is called against a freed ONNX session. Locking is
    // cheap here — transcription holds the monitor for ~200–500 ms, and
    // release blocks the caller (Main thread on idle UI) only briefly.
    private val nativeLock = Any()

    // Set to true once release() has destroyed the underlying ONNX session.
    // Anyone who already holds (or later acquires) a stale reference to this
    // OfflineTranscriber instance must check this flag inside the nativeLock
    // before touching `recognizer` — otherwise we get a use-after-free SEGV.
    @Volatile private var isReleased = false

    companion object {
        private const val TAG = "OfflineTranscriber"

        @Volatile private var instance: OfflineTranscriber? = null

        suspend fun getInstance(context: Context, onReady: (Long, Boolean) -> Unit = { _, _ -> }): OfflineTranscriber = withContext(Dispatchers.IO) {
            instance?.let { onReady(0, true); return@withContext it }
            synchronized(this@Companion) {
                instance?.let { onReady(0, true); return@synchronized it }
                val started = android.os.SystemClock.elapsedRealtime()
                val (recognizer, provider) = buildRecognizer(context)
                OfflineTranscriber(recognizer, provider).also {
                    instance = it
                    onReady(android.os.SystemClock.elapsedRealtime() - started, false)
                }
            }
        }

        // Called on the benchmark worker. Do not transfer a newly allocated native
        // resource across a cancellable dispatcher boundary before the caller owns it.
        internal fun createDiagnostic(context: Context): OfflineTranscriber {
            val (recognizer, provider) = buildRecognizer(context)
            return OfflineTranscriber(recognizer, provider)
        }

        fun release() {
            synchronized(this) {
                val toRelease = instance ?: return
                instance = null
                // Take the per-instance native lock so any in-flight
                // transcription finishes before we destroy the ONNX session.
                // Mark as released *inside* the lock so any caller who got
                // here later (via a cached reference) sees the flag and bails
                // instead of touching the freed recognizer.
                synchronized(toRelease.nativeLock) {
                    toRelease.isReleased = true
                    toRelease.recognizer.release()
                }
            }
        }

        private fun buildRecognizer(context: Context): Pair<OfflineRecognizer, String> {
            val dir = GigaAmModel.modelDir(context)
            val missing = GigaAmModel.FILES.map { File(dir, it.name) }.filter { !it.exists() }
            if (missing.isNotEmpty()) {
                throw IllegalStateException("Model files missing: ${missing.map { it.name }}")
            }

            // Pixel evaluation found no repeatable NNAPI gain and did find
            // intermittent empty transcripts. Keep the reviewed production
            // contract deterministic until a future backend passes the same
            // corpus and rapid-toggle gates.
            val provider = "cpu"
            val recognizer = OfflineRecognizer(null, buildConfig(dir, provider))
            Log.i(TAG, "GigaAM v3 provider=$provider — OK")
            return Pair(recognizer, provider)
        }

        private fun buildConfig(dir: File, provider: String): OfflineRecognizerConfig {
            val modelConfig = OfflineModelConfig(
                transducer = OfflineTransducerModelConfig(
                    encoder = File(dir, GigaAmModel.ENCODER).absolutePath,
                    decoder = File(dir, GigaAmModel.DECODER).absolutePath,
                    joiner = File(dir, GigaAmModel.JOINER).absolutePath
                ),
                tokens = File(dir, GigaAmModel.TOKENS).absolutePath,
                numThreads = 4,
                provider = provider,
                modelType = "nemo_transducer"
            )
            return OfflineRecognizerConfig(
                featConfig = FeatureConfig(sampleRate = 16000, featureDim = 80),
                modelConfig = modelConfig
            )
        }
    }

    internal fun closeDiagnostic() {
        synchronized(nativeLock) {
            if (!isReleased) { isReleased = true; recognizer.release() }
        }
    }

    override suspend fun transcribe(
        pcmData: ByteArray,
        rate: Int,
        width: Int,
        channels: Int,
    ): String = decode(pcmData, withWords = false).text

    override suspend fun transcribeTimed(
        pcmData: ByteArray,
        rate: Int,
        width: Int,
        channels: Int,
    ): TimedTranscript = decode(pcmData, withWords = true)

    private suspend fun decode(
        pcmData: ByteArray,
        withWords: Boolean,
    ): TimedTranscript = withContext(Dispatchers.Default) {
        if (pcmData.isEmpty()) return@withContext TimedTranscript("", emptyList())

        val buf = ByteBuffer.wrap(pcmData).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
        val samples = FloatArray(buf.remaining()) { buf.get().toFloat() / 32768f }

        // Hold the native lock for the entire critical section. release()
        // can run at any time (IME hidden, onTrimMemory) and would crash
        // the JNI layer if it freed the ONNX session mid-call. Inside the
        // lock we also check isReleased — VadRecorder may have captured
        // this instance before release() ran, in which case we must NOT
        // dereference the freed recognizer.
        val transcript = synchronized(nativeLock) {
            if (isReleased) {
                Log.w(TAG, "transcribe called on released instance — dropping ${pcmData.size}B")
                return@synchronized TimedTranscript("", emptyList())
            }
            val stream = recognizer.createStream()
            stream.acceptWaveform(samples, sampleRate = 16000)
            recognizer.decode(stream)
            val result = recognizer.getResult(stream)
            stream.release()
            val words =
                if (withWords) {
                    WordTimestamps.fromTokens(result.tokens, result.timestamps, samples.size * 1000L / 16000)
                } else {
                    emptyList()
                }
            TimedTranscript(result.text.trim(), words)
        }
        val text = transcript.text

        // Privacy: log only the length, never the recognised text content.
        // logcat is visible to anyone who runs `adb logcat` while the app is
        // active and persists in the system ring buffer; even though it's
        // volatile, leaking the user's dictated text there violates the
        // "nothing leaves the device" promise that's the project's main USP.
        Log.i(TAG, "Offline transcript: ${text.length} chars")
        transcript
    }

}
