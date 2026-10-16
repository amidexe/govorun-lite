package com.govorun.lite.transcriber

interface Transcriber {
    suspend fun transcribe(
        pcmData: ByteArray,
        rate: Int = 16000,
        width: Int = 2,
        channels: Int = 1,
    ): String
}
