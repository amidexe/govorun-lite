package com.govorun.lite.api

internal object ContinuousRecognitionProfile {
    fun minimumSilenceSeconds(
        profile: Int,
        defaultSeconds: Float,
    ): Float =
        when (profile and RecognitionApi.CONTINUOUS_PROFILE_MASK) {
            RecognitionApi.CONTINUOUS_PROFILE_TASK_FLOW -> RecognitionApi.TASK_FLOW_MIN_SILENCE_SECONDS
            else -> defaultSeconds
        }

    fun wordTimestampsRequested(profile: Int): Boolean =
        profile and RecognitionApi.CONTINUOUS_FLAG_WORD_TIMESTAMPS != 0

    fun dictationSignalRequested(profile: Int): Boolean =
        profile and RecognitionApi.CONTINUOUS_FLAG_DICTATION_SIGNAL != 0
}
