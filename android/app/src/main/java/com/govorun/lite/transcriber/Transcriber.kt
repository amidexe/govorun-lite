package com.govorun.lite.transcriber

interface Transcriber {
    suspend fun transcribe(
        pcmData: ByteArray,
        rate: Int = 16000,
        width: Int = 2,
        channels: Int = 1,
    ): String

    /** Same recognition pass as [transcribe], additionally returning word times. */
    suspend fun transcribeTimed(
        pcmData: ByteArray,
        rate: Int = 16000,
        width: Int = 2,
        channels: Int = 1,
    ): TimedTranscript = TimedTranscript(transcribe(pcmData, rate, width, channels), emptyList())
}
