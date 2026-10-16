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
    const val CAPABILITIES =
        CAPABILITY_CONTINUOUS_PROFILES or
            CAPABILITY_PRIMARY_APP_PREEMPTION or
            CAPABILITY_LONG_FORM_FILES
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
    // An implementation threshold, not a public duration limit. Larger finite
    // files transparently use the segmenting long-form pipeline.
    const val IN_MEMORY_AUDIO_SECONDS = 120
    const val IN_MEMORY_AUDIO_BYTES =
        SAMPLE_RATE_HZ * CHANNEL_COUNT * (BITS_PER_SAMPLE / 8) * IN_MEMORY_AUDIO_SECONDS
    // Keeps the final oneway Binder callback safely below its transaction cap.
    const val MAX_FINAL_RESULT_CHARS = 400_000
    const val MAX_INFERENCE_MILLIS = 45_000L
    const val SCOPE_LOCAL_RECOGNITION = 1
}
