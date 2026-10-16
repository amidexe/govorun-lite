package com.govorun.lite.service

import android.os.SystemClock
import com.govorun.lite.stylization.DiagnosticAttempt
import com.govorun.lite.stats.StatsStore

/** Adjacent monotonic boundaries, ending when the editor confirms insertion. */
internal class DictationTiming(private val now: () -> Long = SystemClock::elapsedRealtimeNanos) {
    private var released = 0L
    private var recognitionStarted = 0L
    private var recognized = 0L
    private var processing = 0L
    private var result = 0L
    private var recognizing = false
    private var audioMillis = 0L
    private var offlineCharacters = 0
    private var offlineWords: Int? = null
    private var outputCharacters = 0
    private var modelMillis = 0L
    private var modelCached = false
    internal var fields: String? = null
        private set

    @Synchronized fun release() {
        if (released != 0L) return
        released = now()
        if (!recognizing) { recognitionStarted = 0; recognized = 0 }
    }
    @Synchronized fun recognize() {
        if (recognitionStarted == 0L) recognitionStarted = now()
        recognizing = true
    }
    @Synchronized fun recognized() { recognized = now(); recognizing = false }
    @Synchronized fun audio(millis: Long) { audioMillis = millis }
    @Synchronized fun model(millis: Long, cached: Boolean) { modelMillis = millis; modelCached = cached }
    @Synchronized fun process(text: String, count: (String) -> Int = StatsStore::countWords) {
        val words = try { count(text) }
        catch (error: java.util.concurrent.CancellationException) { throw error }
        catch (_: Exception) { null }
        process(text.length, words)
    }
    @Synchronized fun process(characters: Int = 0, words: Int? = null) {
        offlineCharacters = characters; offlineWords = words; processing = now()
    }
    @Synchronized fun result(characters: Int = 0) { outputCharacters = characters; result = now() }

    @Synchronized fun inserted() {
        if (released == 0L || processing == 0L || result == 0L || fields != null) return
        val end = now()
        val boundaries = listOf(released, recognitionStarted, recognized, processing, result, end)
            .runningFold(released) { previous, time -> time.coerceIn(previous, end) }.drop(1)
            .map { (it - released) / 1_000_000 }
        val names = listOf("tail_ms", "recognition_ms", "delivery_prep_ms", "processing_ms", "insertion_ms")
        fields = "full_ms=${boundaries.last()} " + names.zip(boundaries.zipWithNext { a, b -> b - a })
            .joinToString(" ") { (name, ms) -> "$name=$ms" } +
            " audio_ms=$audioMillis offline_chars=$offlineCharacters" + (offlineWords?.let { " offline_words=$it" } ?: "") +
            " output_chars=$outputCharacters" +
            " model_load_ms=$modelMillis model_cached=${if (modelCached) 1 else 0}"
        DiagnosticAttempt.current?.recordDeliveryTiming(checkNotNull(fields))
    }
}
