package com.govorun.lite.api

import android.content.Context
import android.os.ParcelFileDescriptor
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.withTimeout

/** Selects an in-memory or segmenting path without exposing model limits to clients. */
internal class OneShotRecognitionRunner(
    private val context: Context,
    private val descriptor: ParcelFileDescriptor,
) {
    suspend fun run(): String {
        val declaredSize = descriptor.statSize
        if (PcmAudioValidator.sizeError(declaredSize) != null) throw InvalidPcmException()
        return if (declaredSize <= RecognitionApi.IN_MEMORY_AUDIO_BYTES) {
            recognizeInMemory()
        } else {
            recognizeLongForm()
        }
    }

    private suspend fun recognizeInMemory(): String {
        val pcm = PcmAudioReader.read(descriptor)
        return withTimeout(RecognitionApi.MAX_INFERENCE_MILLIS) {
            OfflineTranscriber
                .getInstance(context)
                .transcribe(pcm, RecognitionApi.SAMPLE_RATE_HZ, 2, 1)
        }.trim()
    }

    private suspend fun recognizeLongForm(): String {
        val transcript = LongFormTranscriptAccumulator()
        ContinuousRecognitionPipeline(
            context = context,
            descriptor = descriptor,
            minimumSilenceSeconds = Prefs.getPauseLengthSeconds(context),
            onReady = {},
            onSpeechState = {},
            onResult = { _, text -> transcript.append(text) },
        ).run()
        return transcript.result()
    }
}
