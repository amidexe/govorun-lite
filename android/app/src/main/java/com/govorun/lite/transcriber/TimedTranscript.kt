package com.govorun.lite.transcriber

/** One recognised word with its position in the audio, in milliseconds from its start. */
data class TimedWord(val text: String, val startMs: Long, val endMs: Long)

data class TimedTranscript(val text: String, val words: List<TimedWord>)

/** Joins SentencePiece tokens into words using the per-token times the model already returns. */
internal object WordTimestamps {
    private const val WORD_MARK = '▁'

    // The transducer reports when a token was emitted, not how long it lasted.
    // A word therefore ends shortly after its last token, or where the next word starts.
    private const val TOKEN_TAIL_MS = 240L

    fun fromTokens(
        tokens: Array<String>,
        timestamps: FloatArray,
        audioMillis: Long,
    ): List<TimedWord> {
        val count = minOf(tokens.size, timestamps.size)
        val texts = ArrayList<String>()
        val starts = ArrayList<Long>()
        val lasts = ArrayList<Long>()
        val current = StringBuilder()
        var start = 0L
        var last = 0L

        fun flush() {
            val word = current.toString().trim()
            if (word.isNotEmpty()) {
                texts.add(word)
                starts.add(start)
                lasts.add(last)
            }
            current.setLength(0)
        }

        for (i in 0 until count) {
            val token = tokens[i]
            val at = (timestamps[i] * 1000f).toLong().coerceIn(0L, audioMillis)
            // sherpa-onnx hands the SentencePiece word mark back either as U+2581 or as a plain space.
            val startsWord = token.startsWith(WORD_MARK) || token.startsWith(' ')
            if (startsWord) flush()
            if (current.isEmpty()) start = at
            current.append(token.replace(WORD_MARK, ' '))
            last = at
        }
        flush()

        val words = ArrayList<TimedWord>(texts.size)
        var floor = 0L
        for (i in texts.indices) {
            val begin = maxOf(starts[i], floor)
            val nextBegin = if (i + 1 < texts.size) starts[i + 1] else audioMillis
            val finish = minOf(nextBegin, lasts[i] + TOKEN_TAIL_MS).coerceIn(begin, maxOf(begin, audioMillis))
            words.add(TimedWord(texts[i], begin, finish))
            floor = begin
        }
        return words
    }
}
