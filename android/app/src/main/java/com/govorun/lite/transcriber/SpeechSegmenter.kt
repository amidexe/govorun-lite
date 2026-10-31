package com.govorun.lite.transcriber

import com.k2fsa.sherpa.onnx.Vad
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Cuts 16 kHz 16-bit mono PCM into Silero's speech segments, each with the sound just before it ([VadOpening]). The bird's
 * dictation ([VadRecorder]) and the recognition Echo's notes use (ContinuousRecognitionPipeline) both cut here and only feed the
 * sound in and take the segments out, so a change of the cutting, or another model behind it, reaches both at once. Positions
 * are absolute samples of the sound given, skipped parts included. Not thread-safe: a shared [vad] needs the caller's lock.
 */
internal class SpeechSegmenter(private val vad: Vad) {
    class Segment(val pcm: ByteArray, val startSample: Long, val speechSamples: Int)

    // The sound since the last segment, with the absolute sample of its first byte.
    private val retained = ByteArrayOutputStream()
    private var retainedStart = 0L
    private var position = 0L
    private var origin = 0L
    private var originPending = false
    private var lastEnd = 0L

    /** Segments cut so far. */
    var emitted = 0
        private set

    /** Whether Silero hears speech now. */
    fun speech(): Boolean = vad.isSpeechDetected()

    /** One window of [size] bytes (the last one may be shorter): into the retained sound and Silero; the segments it ended. */
    fun accept(window: ByteArray, size: Int = window.size): List<Segment> {
        if (originPending) {
            origin = position
            originPending = false
        }
        if (retained.size() == 0) retainedStart = position
        if (retained.size() + size > MAX_RETAINED_BYTES) {
            // Memory stays bounded: the older half goes. Silero cuts long before this on natural speech.
            val dropped = retained.size() / 2 / 2 * 2
            val kept = retained.toByteArray().copyOfRange(dropped, retained.size())
            retained.reset()
            retained.write(kept)
            retainedStart += dropped / 2
        }
        retained.write(window, 0, size)
        val shorts = ByteBuffer.wrap(window, 0, size).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer()
        // A short window is fed too, padded only at its end.
        val samples = FloatArray(WINDOW_SAMPLES) { if (shorts.hasRemaining()) shorts.get().toFloat() / 32768f else 0f }
        vad.acceptWaveform(samples)
        position += WINDOW_SAMPLES
        return drain()
    }

    /** Sound that is not cut (Echo's stream while a dictation holds it): only its place counts. */
    fun skip(size: Int) {
        position += size / 2
    }

    /** The end of the sound: what Silero still holds. */
    fun flush(): List<Segment> {
        vad.flush()
        return drain()
    }

    /** Silero starts over where the sound goes on; what was retained is dropped. */
    fun restart() {
        vad.reset()
        retained.reset()
        originPending = true
    }

    /** The sound retained since the last segment, where it starts and its size. */
    fun rest(): ByteArray = retained.toByteArray()

    fun restStart(): Long = retainedStart

    fun restBytes(): Int = retained.size()

    private fun drain(): List<Segment> {
        if (vad.empty()) return emptyList()
        val segments = ArrayList<Segment>(1)
        while (!vad.empty()) {
            val segment = vad.front()
            vad.pop()
            val start = origin + segment.start
            val retainedEnd = retainedStart + retained.size() / 2
            val opening = VadOpening.range(emitted == 0, start, retainedStart, retainedEnd, lastEnd)
            segments += Segment(VadOpening.pcm(opening, retained, retainedStart, segment.samples),
                if (opening.isEmpty()) start else opening.first, segment.samples.size)
            // Silence confirmed: everything retained so far went with the segment.
            retained.reset()
            retainedStart = retainedEnd
            lastEnd = start + segment.samples.size
            emitted++
        }
        return segments
    }

    companion object {
        const val WINDOW_SAMPLES = 512
        const val MAX_RETAINED_BYTES = VadRecorder.SAMPLE_RATE * 2 * 60
    }
}
