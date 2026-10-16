package com.govorun.lite.ui

internal object DictionaryVoiceAwakePolicy {
    fun shouldKeepAwake(
        recording: Boolean,
        processing: Boolean,
        speechActive: Boolean,
        keepForWholeRecording: Boolean,
    ): Boolean = processing || (recording && (speechActive || keepForWholeRecording))
}
