package com.govorun.lite.api

import android.content.Context
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.util.Log
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.k2fsa.sherpa.onnx.Vad
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

/** Reads client-owned PCM, applies Govorun's Silero VAD, and transcribes each utterance. */
internal class ContinuousRecognitionPipeline(
    private val context: Context,
    private val descriptor: ParcelFileDescriptor,
    private val minimumSilenceSeconds: Float,
    private val onReady: () -> Unit,
    private val onSpeechState: (Boolean) -> Unit,
    private val onResult: (Long, String) -> Unit,
) {
    suspend fun run() =
        withContext(Dispatchers.IO) {
            val vad = VadRecorder.createSessionVad(context, minimumSilenceSeconds)
            val transcriber = OfflineTranscriber.getInstance(context)
            try {
                vad.reset()
                onReady()
                ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { input ->
                    coroutineScope {
                        val segments = Channel<ByteArray>(SEGMENT_QUEUE_CAPACITY)
                        val reader = launch { readSegments(input, vad, segments) }
                        var sequence = 0L
                        try {
                            for (pcm in segments) {
                                val startedAt = SystemClock.elapsedRealtime()
                                val text =
                                    transcriber
                                        .transcribe(pcm, SAMPLE_RATE, BYTES_PER_SAMPLE, CHANNEL_COUNT)
                                        .trim()
                                Log.i(
                                    TAG,
                                    "Streaming segment ${pcm.size / BYTES_PER_MILLISECOND}ms " +
                                        "transcribed in ${SystemClock.elapsedRealtime() - startedAt}ms (${text.length} chars)",
                                )
                                if (text.isNotEmpty()) onResult(++sequence, text)
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
        segments: Channel<ByteArray>,
    ) {
        val window = ByteArray(WINDOW_BYTES)
        val tail = ByteArrayOutputStream()
        var speechActive = false
        var emitted = 0

        suspend fun emitVadSegments() {
            while (!vad.empty()) {
                val segment = vad.front()
                vad.pop()
                val pcm = ByteArray(segment.samples.size * BYTES_PER_SAMPLE)
                val output = ByteBuffer.wrap(pcm).order(ByteOrder.LITTLE_ENDIAN)
                segment.samples.forEach { sample ->
                    output.putShort((sample * 32767f).toInt().coerceIn(-32768, 32767).toShort())
                }
                // Backpressure is required for prepared files: they can be
                // read much faster than the model transcribes them. Suspending
                // here bounds memory without imposing a duration limit.
                segments.send(pcm)
                tail.reset()
                emitted++
            }
        }

        try {
            while (kotlin.coroutines.coroutineContext.isActive) {
                val bytesRead = input.readWindow(window)
                if (bytesRead == 0) break
                if (bytesRead < WINDOW_BYTES) {
                    tail.write(window, 0, bytesRead)
                    break
                }
                if (tail.size() + WINDOW_BYTES > MAX_TAIL_BYTES) {
                    val current = tail.toByteArray()
                    tail.reset()
                    tail.write(current, current.size / 2, current.size - current.size / 2)
                }
                tail.write(window)
                val shorts = ByteBuffer.wrap(window).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
                val samples = FloatArray(VAD_WINDOW_SAMPLES) { shorts.get().toFloat() / 32768f }
                vad.acceptWaveform(samples)
                emitVadSegments()
                val nextSpeechActive = vad.isSpeechDetected()
                if (nextSpeechActive != speechActive) {
                    speechActive = nextSpeechActive
                    onSpeechState(speechActive)
                }
            }
        } finally {
            if (speechActive) onSpeechState(false)
            val beforeFlush = emitted
            vad.flush()
            emitVadSegments()
            if (emitted == beforeFlush && tail.size() >= MIN_TAIL_BYTES) {
                segments.send(tail.toByteArray())
            }
            tail.reset()
            segments.close()
        }
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
        const val MAX_TAIL_BYTES = SAMPLE_RATE * BYTES_PER_SAMPLE * 60
        const val SEGMENT_QUEUE_CAPACITY = 16
    }
}
