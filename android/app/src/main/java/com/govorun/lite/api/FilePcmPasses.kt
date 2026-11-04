package com.govorun.lite.api

import java.io.InputStream
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive

/** Bounded file passes. Quiet spots choose boundaries; no detector may discard the sound between them. */
internal object FilePcmPasses {
    const val MAX_BYTES = RecognitionApi.IN_MEMORY_AUDIO_BYTES
    private const val CUT_SEARCH_BYTES = 5 * RecognitionApi.SAMPLE_RATE_HZ * 2
    private const val CUT_STEP_BYTES = RecognitionApi.SAMPLE_RATE_HZ * 2 / 50
    private const val BYTES_PER_SAMPLE = 2

    suspend fun read(input: InputStream, emit: suspend (ByteArray, Long) -> Unit) {
        val buffer = ByteArray(MAX_BYTES + BYTES_PER_SAMPLE)
        var held = 0
        var startSample = 0L
        var ended = false
        while (!ended || held > 0) {
            while (!ended && held < buffer.size) {
                currentCoroutineContext().ensureActive()
                val n = input.read(buffer, held, buffer.size - held)
                if (n < 0) ended = true else held += n
            }
            if (held == 0) break
            if (held % BYTES_PER_SAMPLE != 0) throw InvalidPcmException()
            val end = if (held <= MAX_BYTES) held else quietestCut(buffer, 0)
            emit(buffer.copyOfRange(0, end), startSample)
            buffer.copyInto(buffer, 0, end, held)
            held -= end
            startSample += end / BYTES_PER_SAMPLE
        }
    }

    /** Ends a pass at the quietest 20 ms in its last 5 s, so a cut rarely splits a word. */
    fun quietestCut(pcm: ByteArray, from: Int): Int {
        val limit = from + MAX_BYTES
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

}
