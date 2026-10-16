package com.govorun.lite.api

internal object PcmAudioValidator {
    fun metadataError(sampleRateHz: Int, channelCount: Int, bitsPerSample: Int): Int? =
        when {
            sampleRateHz != RecognitionApi.SAMPLE_RATE_HZ -> RecognitionApi.ERROR_INVALID_AUDIO
            channelCount != RecognitionApi.CHANNEL_COUNT -> RecognitionApi.ERROR_INVALID_AUDIO
            bitsPerSample != RecognitionApi.BITS_PER_SAMPLE -> RecognitionApi.ERROR_INVALID_AUDIO
            else -> null
        }

    fun sizeError(sizeBytes: Long): Int? =
        when {
            sizeBytes <= 0L || sizeBytes % 2L != 0L -> RecognitionApi.ERROR_INVALID_AUDIO
            else -> null
        }
}
