package com.govorun.lite.api

import android.content.Context
import android.os.ParcelFileDescriptor
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.PhraseQueue
import com.govorun.lite.transcriber.RecognitionSessionCoordinator
import com.govorun.lite.transcriber.TimedWord
import com.govorun.lite.transcriber.TimedTranscript
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.withTimeout

/** Selects an in-memory or segmenting path without exposing model limits to clients. */
internal class OneShotRecognitionRunner(
    private val context: Context,
    private val descriptor: ParcelFileDescriptor,
) {
    suspend fun run(): String = runTimed(withWords = false).text

    suspend fun runTimed(withWords: Boolean = true): TimedTranscript {
        val declaredSize = descriptor.statSize
        if (PcmAudioValidator.sizeError(declaredSize) != null) throw InvalidPcmException()
        return if (declaredSize <= RecognitionApi.IN_MEMORY_AUDIO_BYTES) {
            recognizeInMemory(withWords)
        } else {
            recognizeLongForm(withWords)
        }
    }

    private suspend fun recognizeInMemory(withWords: Boolean): TimedTranscript {
        val pcm = PcmAudioReader.read(descriptor)
        val transcriber = OfflineTranscriber.getInstance(context)
        // A clip of one phrase is usually push-to-talk: the person waits for it right now.
        return RecognitionSessionCoordinator.phrases.decode(PhraseQueue.Kind.LIVE) {
            withTimeout(RecognitionApi.MAX_INFERENCE_MILLIS) {
                if (withWords) {
                    transcriber.transcribeTimed(pcm, RecognitionApi.SAMPLE_RATE_HZ, 2, 1)
                } else {
                    TimedTranscript(transcriber.transcribe(pcm, RecognitionApi.SAMPLE_RATE_HZ, 2, 1), emptyList())
                }
            }
        }.let { TimedTranscript(it.text.trim(), it.words) }
    }

    private suspend fun recognizeLongForm(withWords: Boolean): TimedTranscript {
        val transcript = LongFormTranscriptAccumulator()
        val timedSink: (Long, String, List<TimedWord>) -> Unit = { _, text, words ->
            transcript.appendTimed(text, words)
        }
        ContinuousRecognitionPipeline(
            context = context,
            descriptor = descriptor,
            minimumSilenceSeconds = Prefs.getPauseLengthSeconds(context),
            onReady = {},
            onSpeechState = {},
            onResult = { _, text -> if (!withWords) transcript.append(text) },
            onTimedResult = timedSink.takeIf { withWords },
            prepared = true,
        ).run()
        return TimedTranscript(transcript.result(), transcript.words())
    }
}
