package com.govorun.lite.api

/** Builds a one-shot long-form result without risking Binder transaction overflow. */
internal class LongFormTranscriptAccumulator(
    private val maximumChars: Int = RecognitionApi.MAX_FINAL_RESULT_CHARS,
) {
    private val value = StringBuilder()

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

    fun result(): String = value.toString()
}

internal class LongFormResultLimitException : IllegalArgumentException()
