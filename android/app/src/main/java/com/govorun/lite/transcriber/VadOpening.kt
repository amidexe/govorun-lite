package com.govorun.lite.transcriber

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * What a speech segment gets from the sound just before it. Silero starts a phrase a little after its first sound, so the
 * first segment keeps everything recorded before it (up to six seconds) and a later one up to 450 ms, never reaching into
 * the previous segment — the desktop's rule. Bench of 29 Oct 2026 («до паузы»): the first word was cut in 5.6% of short
 * phrases without it and in none with it; recording with the button and noise unchanged. Dictation and the recognition
 * that Echo's notes use both take a segment through here, so a fix of one is a fix of the other.
 */
internal object VadOpening {
    const val FIRST_MAX_SAMPLES = VadRecorder.SAMPLE_RATE * 6L
    const val PREROLL_SAMPLES = VadRecorder.SAMPLE_RATE * 450L / 1000

    /** Absolute samples to put before a segment starting at [start], within the retained [tailStart] until [tailEnd]. */
    fun range(first: Boolean, start: Long, tailStart: Long, tailEnd: Long, lastEnd: Long): LongRange {
        val from = when {
            !first -> maxOf(start - PREROLL_SAMPLES, lastEnd)
            start in 1..FIRST_MAX_SAMPLES -> 0L
            else -> return LongRange.EMPTY
        }
        val begin = maxOf(from, tailStart)
        val end = minOf(start, tailEnd)
        return if (begin < end) begin until end else LongRange.EMPTY
    }

    /** The segment as 16-bit PCM with [opening] (from [range]) taken out of [tail], whose first byte is sample [tailStart]. */
    fun pcm(opening: LongRange, tail: ByteArrayOutputStream, tailStart: Long, samples: FloatArray): ByteArray {
        val openingBytes = if (opening.isEmpty()) 0 else ((opening.last - opening.first + 1) * 2).toInt()
        val pcm = ByteArray(samples.size * 2 + openingBytes)
        if (openingBytes > 0) {
            val from = ((opening.first - tailStart) * 2).toInt()
            tail.toByteArray().copyInto(pcm, 0, from, from + openingBytes)
        }
        val buffer = ByteBuffer.wrap(pcm).order(ByteOrder.LITTLE_ENDIAN)
        buffer.position(openingBytes)
        for (sample in samples) buffer.putShort((sample * 32767f).toInt().coerceIn(-32768, 32767).toShort())
        return pcm
    }
}
