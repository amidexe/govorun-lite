package com.govorun.lite.api

import android.content.Context
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.util.Log
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.PhraseQueue
import com.govorun.lite.transcriber.RecognitionSessionCoordinator
import com.govorun.lite.transcriber.TimedWord
import com.govorun.lite.transcriber.SpeechSegmenter
import com.govorun.lite.transcriber.VadRecorder
import com.k2fsa.sherpa.onnx.Vad
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.InputStream

/** Reads client-owned PCM, applies Govorun's Silero VAD, and transcribes each utterance. */
internal class ContinuousRecognitionPipeline(
    private val context: Context,
    private val descriptor: ParcelFileDescriptor,
    private val minimumSilenceSeconds: Float,
    private val onReady: () -> Unit,
    private val onSpeechState: (Boolean) -> Unit,
    private val onResult: (Long, String) -> Unit,
    /** When set, each segment is decoded with word times (from the same pass) and reported here. */
    private val onTimedResult: ((Long, String, List<TimedWord>) -> Unit)? = null,
    /** A finished recording: its phrases wait behind live streams in [PhraseQueue]. */
    private val prepared: Boolean = false,
    /** Set when the client asked to ride out Govorun's dictation: audio is skipped and phrases wait. */
    private val hold: DictationHold? = null,
    /** A recording's phrases wait here while Govorun's dictation holds the engine; none of its audio is skipped. */
    private val yieldToDictation: DictationHold? = null,
) {
    private class Segment(val pcm: ByteArray, val startSample: Long)

    // A stream read well ahead of real time is a recording being replayed, not a live microphone,
    // so a client cannot jump the queue by sending a file through the continuous API.
    @Volatile private var aheadOfRealTime = false

    /** A finished recording, sent or read ahead of real time: it waits for a dictation instead of being stopped by it. */
    val isRecording: Boolean get() = prepared || aheadOfRealTime

    suspend fun run() =
        withContext(Dispatchers.IO) {
            val vad = VadRecorder.createSessionVad(context, minimumSilenceSeconds)
            val transcriber = OfflineTranscriber.getInstance(context)
            try {
                vad.reset()
                onReady()
                ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { input ->
                    coroutineScope {
                        val segments = Channel<Segment>(SEGMENT_QUEUE_CAPACITY)
                        val reader = launch { readSegments(input, vad, segments) }
                        var sequence = 0L
                        try {
                            for (segment in segments) {
                                hold?.awaitReleased()
                                if (isRecording) yieldToDictation?.awaitReleased()
                                val pcm = segment.pcm
                                val startedAt = SystemClock.elapsedRealtime()
                                val offsetMs = segment.startSample * 1_000L / SAMPLE_RATE
                                val timed = onTimedResult != null
                                val kind =
                                    if (prepared || aheadOfRealTime) PhraseQueue.Kind.FILE else PhraseQueue.Kind.LIVE
                                val (transcript, text) =
                                    RecognitionSessionCoordinator.phrases.decode(kind) {
                                        if (timed) {
                                            val result =
                                                transcriber.transcribeTimed(pcm, SAMPLE_RATE, BYTES_PER_SAMPLE, CHANNEL_COUNT)
                                            result to result.text
                                        } else {
                                            null to
                                                transcriber
                                                    .transcribe(pcm, SAMPLE_RATE, BYTES_PER_SAMPLE, CHANNEL_COUNT)
                                                    .trim()
                                        }
                                    }
                                Log.i(
                                    TAG,
                                    "Streaming segment ${pcm.size / BYTES_PER_MILLISECOND}ms " +
                                        "transcribed in ${SystemClock.elapsedRealtime() - startedAt}ms (${text.length} chars)",
                                )
                                if (text.isNotEmpty()) {
                                    sequence++
                                    onResult(sequence, text)
                                    if (transcript != null) {
                                        onTimedResult?.invoke(
                                            sequence,
                                            text,
                                            transcript.words.map {
                                                TimedWord(it.text, it.startMs + offsetMs, it.endMs + offsetMs)
                                            },
                                        )
                                    }
                                }
                            }
                        } finally {
                            reader.cancelAndJoin()
                        }
                    }
                }
            } finally {
                runCatching { vad.release() }
            }
        }

    private suspend fun readSegments(
        input: InputStream,
        vad: Vad,
        segments: Channel<Segment>,
    ) {
        val window = ByteArray(WINDOW_BYTES)
        // The cutting is the bird's own (SpeechSegmenter): this reader only feeds the client's sound in. Times are positions
        // in the client's audio from the stream start and never restart, the parts skipped during a dictation included.
        val segmenter = SpeechSegmenter(vad)
        var speechActive = false
        var readBytes = 0L
        var firstReadAt = 0L
        var skipping = false
        var seenDictations = hold?.generation ?: 0L

        // Backpressure is required for prepared files: they can be
        // read much faster than the model transcribes them. Suspending
        // here bounds memory without imposing a duration limit.
        suspend fun send(cut: List<SpeechSegmenter.Segment>) {
            for (segment in cut) segments.send(Segment(segment.pcm, segment.startSample))
        }

        try {
            while (kotlin.coroutines.coroutineContext.isActive) {
                val bytesRead = input.readWindow(window)
                if (bytesRead == 0) break
                readBytes += bytesRead
                val now = SystemClock.elapsedRealtime()
                if (firstReadAt == 0L) firstReadAt = now
                aheadOfRealTime = readBytes / BYTES_PER_MILLISECOND - (now - firstReadAt) > PREPARED_LEAD_MS
                if (bytesRead < WINDOW_BYTES) {
                    send(segmenter.accept(window, bytesRead))
                    break
                }
                val held = hold?.isHeld == true
                val dictations = hold?.generation ?: 0L
                if ((held || dictations != seenDictations) && !skipping) {
                    // A dictation ends the open phrase: what was said before it is recognized now,
                    // even when the client sent nothing while the bird listened and speaks on at once.
                    // The dictation itself is never read.
                    send(segmenter.flush())
                    segmenter.restart()
                    if (speechActive) {
                        speechActive = false
                        onSpeechState(false)
                    }
                }
                seenDictations = dictations
                skipping = held
                if (held) {
                    segmenter.skip(bytesRead)
                    continue
                }
                send(segmenter.accept(window))
                val nextSpeechActive = segmenter.speech()
                if (nextSpeechActive != speechActive) {
                    speechActive = nextSpeechActive
                    onSpeechState(speechActive)
                }
            }
        } finally {
            if (speechActive) onSpeechState(false)
            val beforeFlush = segmenter.emitted
            send(segmenter.flush())
            if (segmenter.emitted == beforeFlush && segmenter.restBytes() >= MIN_TAIL_BYTES) {
                // The rest is the audio since the last segment, ending at the last byte read.
                // It is decoded in passes no longer than a VAD segment: one long pass made ONNX keep
                // hundreds of megabytes it never returns.
                val restStartSample = segmenter.restStart()
                val pcm = segmenter.rest()
                var from = 0
                while (from < pcm.size) {
                    val to = quietestCut(pcm, from)
                    segments.send(Segment(pcm.copyOfRange(from, to), restStartSample + from / BYTES_PER_SAMPLE))
                    from = to
                }
            }
            segments.close()
        }
    }

    /** Ends a pass at the quietest 20 ms in its last 5 s, so a cut rarely splits a word. */
    private fun quietestCut(pcm: ByteArray, from: Int): Int {
        val limit = from + MAX_PASS_BYTES
        if (limit >= pcm.size) return pcm.size
        var best = limit
        var bestEnergy = Long.MAX_VALUE
        var start = limit - CUT_SEARCH_BYTES
        while (start + CUT_STEP_BYTES <= limit) {
            var energy = 0L
            for (i in start until start + CUT_STEP_BYTES step BYTES_PER_SAMPLE) {
                val sample = (pcm[i].toInt() and 0xFF) or (pcm[i + 1].toInt() shl 8)
                energy += kotlin.math.abs(sample.toShort().toInt())
            }
            if (energy < bestEnergy) { bestEnergy = energy; best = start + CUT_STEP_BYTES / 2 }
            start += CUT_STEP_BYTES
        }
        return best - best % BYTES_PER_SAMPLE
    }

    private fun InputStream.readWindow(target: ByteArray): Int {
        var offset = 0
        while (offset < target.size) {
            val read = read(target, offset, target.size - offset)
            if (read < 0) break
            if (read == 0) continue
            offset += read
        }
        return offset
    }

    private companion object {
        const val TAG = "ContinuousRecognition"
        const val SAMPLE_RATE = 16_000
        const val CHANNEL_COUNT = 1
        const val BYTES_PER_SAMPLE = 2
        const val VAD_WINDOW_SAMPLES = 512
        const val WINDOW_BYTES = VAD_WINDOW_SAMPLES * BYTES_PER_SAMPLE
        const val BYTES_PER_MILLISECOND = SAMPLE_RATE * BYTES_PER_SAMPLE / 1_000
        const val MIN_TAIL_BYTES = SAMPLE_RATE * BYTES_PER_SAMPLE / 4
        const val MAX_PASS_BYTES = RecognitionApi.MAX_PASS_SECONDS * SAMPLE_RATE * BYTES_PER_SAMPLE
        const val CUT_SEARCH_BYTES = 5 * SAMPLE_RATE * BYTES_PER_SAMPLE
        const val CUT_STEP_BYTES = SAMPLE_RATE * BYTES_PER_SAMPLE / 50
        // Live phrases go first in PhraseQueue and never pile up; for a file a few phrases of read-ahead
        // are enough, and every open session holds this many phrases of PCM.
        const val SEGMENT_QUEUE_CAPACITY = 4
        const val PREPARED_LEAD_MS = 10_000L
    }
}
