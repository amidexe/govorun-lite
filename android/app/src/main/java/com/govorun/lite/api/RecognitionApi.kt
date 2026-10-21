package com.govorun.lite.api

object RecognitionApi {
    const val SERVICE_ACTION = "com.govorun.api.RECOGNITION_SERVICE"
    const val PROTOCOL_VERSION = 1

    const val AUTHORIZATION_REQUIRED = 0
    const val AUTHORIZATION_GRANTED = 1

    const val CONTINUOUS_PROFILE_DEFAULT = 0
    const val CONTINUOUS_PROFILE_TASK_FLOW = 1
    const val CAPABILITY_CONTINUOUS_PROFILES = 1
    const val CAPABILITY_PRIMARY_APP_PREEMPTION = 1 shl 1
    const val CAPABILITY_LONG_FORM_FILES = 1 shl 2
    const val CAPABILITY_WORD_TIMESTAMPS = 1 shl 3
    // Several external sessions may be open at once and share the engine phrase by phrase:
    // live streams first, then files (PhraseQueue). Without it a second session gets ERROR_BUSY.
    const val CAPABILITY_SHARED_ENGINE = 1 shl 4
    // A stream that opts in is told when Govorun's own dictation starts and ends instead of being preempted.
    const val CAPABILITY_DICTATION_SIGNAL = 1 shl 5
    const val CAPABILITIES =
        CAPABILITY_CONTINUOUS_PROFILES or
            CAPABILITY_PRIMARY_APP_PREEMPTION or
            CAPABILITY_LONG_FORM_FILES or
            CAPABILITY_WORD_TIMESTAMPS or
            CAPABILITY_SHARED_ENGINE or
            CAPABILITY_DICTATION_SIGNAL
    // OR-ed into the profile of recognizeContinuousWithProfile to receive onFinalResultTimed.
    const val CONTINUOUS_FLAG_WORD_TIMESTAMPS = 1 shl 16
    const val CONTINUOUS_FLAG_DICTATION_SIGNAL = 1 shl 17
    const val CONTINUOUS_PROFILE_MASK = 0xFFFF
    // Keeps a one-shot timed result (three parallel arrays) below the Binder transaction cap.
    const val MAX_TIMED_RESULT_WORDS = 10_000
    const val TASK_FLOW_MIN_SILENCE_SECONDS = 0.5f

    const val ERROR_UNAUTHORIZED = 1
    const val ERROR_BUSY = 2
    const val ERROR_MODEL_UNAVAILABLE = 3
    const val ERROR_INVALID_AUDIO = 4
    const val ERROR_RECOGNITION_FAILED = 5
    const val ERROR_UNSUPPORTED_VERSION = 6
    const val ERROR_CANCELED = 7
    const val ERROR_LIMIT_EXCEEDED = 8
    const val ERROR_PREEMPTED = 9

    const val SAMPLE_RATE_HZ = 16_000
    const val CHANNEL_COUNT = 1
    const val BITS_PER_SAMPLE = 16
    // The longest audio decoded in one model pass. ONNX Runtime keeps the peak working memory of
    // its largest pass for the life of the process: one 110 s request grew the native heap from
    // 0.62 to 1.14 GB (emulator, 20 Oct 2026). Equal to the VAD segment cap, so every client
    // request, however long, stays within the memory of ordinary dictation.
    const val MAX_PASS_SECONDS = 30
    // An implementation threshold, not a public duration limit. Larger finite
    // files transparently use the segmenting long-form pipeline.
    const val IN_MEMORY_AUDIO_SECONDS = MAX_PASS_SECONDS
    const val IN_MEMORY_AUDIO_BYTES =
        SAMPLE_RATE_HZ * CHANNEL_COUNT * (BITS_PER_SAMPLE / 8) * IN_MEMORY_AUDIO_SECONDS
    // Keeps the final oneway Binder callback safely below its transaction cap.
    const val MAX_FINAL_RESULT_CHARS = 400_000
    const val MAX_INFERENCE_MILLIS = 45_000L
    const val SCOPE_LOCAL_RECOGNITION = 1
}
