package com.govorun.lite.api

internal object ContinuousRecognitionProfile {
    fun minimumSilenceSeconds(
        profile: Int,
        defaultSeconds: Float,
    ): Float =
        when (profile) {
            RecognitionApi.CONTINUOUS_PROFILE_TASK_FLOW -> RecognitionApi.TASK_FLOW_MIN_SILENCE_SECONDS
            else -> defaultSeconds
        }
}
