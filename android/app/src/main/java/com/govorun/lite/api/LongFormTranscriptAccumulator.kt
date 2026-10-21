package com.govorun.lite.api

import com.govorun.lite.transcriber.TimedWord

/** Builds a one-shot long-form result without risking Binder transaction overflow. */
internal class LongFormTranscriptAccumulator(
    private val maximumChars: Int = RecognitionApi.MAX_FINAL_RESULT_CHARS,
) {
    private val value = StringBuilder()
    private val timedWords = ArrayList<TimedWord>()

    fun append(segment: String) {
        val text = segment.trim()
        if (text.isEmpty()) return
        val separatorLength = if (value.isEmpty()) 0 else 1
        if (value.length + separatorLength + text.length > maximumChars) {
            throw LongFormResultLimitException()
        }
        if (separatorLength != 0) value.append(' ')
        value.append(text)
    }

    /** Appends a segment and its already offset word times; the word count is capped for Binder. */
    fun appendTimed(segment: String, words: List<TimedWord>) {
        if (timedWords.size + words.size > RecognitionApi.MAX_TIMED_RESULT_WORDS) {
            throw LongFormResultLimitException()
        }
        append(segment)
        timedWords.addAll(words)
    }

    fun result(): String = value.toString()

    fun words(): List<TimedWord> = timedWords
}

internal class LongFormResultLimitException : IllegalArgumentException()
