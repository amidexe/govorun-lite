package com.govorun.lite.api

import com.govorun.lite.transcriber.TimedTranscript
import com.govorun.lite.transcriber.TimedWord
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.os.ParcelFileDescriptor
import com.govorun.api.v1.IContinuousRecognitionCallback
import com.govorun.api.v1.IRecognitionCallback
import com.govorun.api.v1.IRecognitionService
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.transcriber.RecognitionSessionArbiter
import com.govorun.lite.transcriber.RecognitionSessionCoordinator
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicBoolean

class RecognitionService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private lateinit var identities: CallerIdentityResolver
    private lateinit var authorizations: RecognitionAuthorizationStore
    private val sessionLock = Any()
    private val sessions = mutableListOf<RecognitionProviderSession>()

    override fun onCreate() {
        super.onCreate()
        RecognitionApiControl.attach(this)
        identities = CallerIdentityResolver(this)
        authorizations = RecognitionAuthorizationStore(this)
    }

    override fun onBind(intent: Intent?): IBinder? =
        binder.takeIf { intent?.action == RecognitionApi.SERVICE_ACTION }

    override fun onDestroy() {
        synchronized(sessionLock) { sessions.toList() }.forEach { it.cancel(notify = false) }
        RecognitionApiControl.detach(this)
        scope.cancel()
        super.onDestroy()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // API v1 is bind-only. Never let an external startService call keep the
        // provider process alive or create a second control surface.
        stopSelf(startId)
        return START_NOT_STICKY
    }

    private val binder =
        object : IRecognitionService.Stub() {
            override fun getProtocolVersion(): Int = RecognitionApi.PROTOCOL_VERSION

            override fun getAuthorizationStatus(clientPackage: String?): Int {
                val identity = authenticatedCaller(clientPackage) ?: return RecognitionApi.AUTHORIZATION_REQUIRED
                return if (authorizations.isAuthorized(identity)) {
                    RecognitionApi.AUTHORIZATION_GRANTED
                } else {
                    RecognitionApi.AUTHORIZATION_REQUIRED
                }
            }

            override fun createAuthorizationRequest(clientPackage: String?): PendingIntent {
                val identity = authenticatedCaller(clientPackage)
                    ?: throw SecurityException("Calling package does not match Binder UID")
                val pending = authorizations.createPending(identity)
                val intent =
                    Intent(this@RecognitionService, RecognitionAuthorizationActivity::class.java)
                        .addFlags(
                            Intent.FLAG_ACTIVITY_NEW_TASK or
                                Intent.FLAG_ACTIVITY_NO_HISTORY or
                                Intent.FLAG_ACTIVITY_EXCLUDE_FROM_RECENTS,
                        )
                        .putExtra(RecognitionAuthorizationActivity.EXTRA_NONCE, pending.nonce)
                        .setIdentifier("govorun-recognition-auth:${pending.nonce}")
                return PendingIntent.getActivity(
                    this@RecognitionService,
                    0,
                    intent,
                    PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_ONE_SHOT,
                )
            }

            override fun recognize(
                clientPackage: String?,
                sessionId: String?,
                pcmAudio: ParcelFileDescriptor?,
                sampleRateHz: Int,
                channelCount: Int,
                bitsPerSample: Int,
                callback: IRecognitionCallback?,
            ) = startOneShotRecognition(
                clientPackage, sessionId, pcmAudio, sampleRateHz, channelCount, bitsPerSample, callback, timed = false,
            )

            override fun recognizeTimed(
                clientPackage: String?,
                sessionId: String?,
                pcmAudio: ParcelFileDescriptor?,
                sampleRateHz: Int,
                channelCount: Int,
                bitsPerSample: Int,
                callback: IRecognitionCallback?,
            ) = startOneShotRecognition(
                clientPackage, sessionId, pcmAudio, sampleRateHz, channelCount, bitsPerSample, callback, timed = true,
            )

            override fun cancel(clientPackage: String?, sessionId: String?) {
                val identity = authenticatedCaller(clientPackage) ?: return
                synchronized(sessionLock) {
                    sessions.firstOrNull {
                        it.uid == identity.uid &&
                            it.packageName == identity.packageName &&
                            it.sessionId == sessionId
                    }
                }?.cancel(notify = true)
            }

            override fun supportsContinuousRecognition(): Boolean = true

            override fun recognizeContinuous(
                clientPackage: String?,
                sessionId: String?,
                pcmStream: ParcelFileDescriptor?,
                sampleRateHz: Int,
                channelCount: Int,
                bitsPerSample: Int,
                callback: IContinuousRecognitionCallback?,
            ) = recognizeContinuousWithProfile(
                clientPackage, sessionId, pcmStream, sampleRateHz, channelCount, bitsPerSample,
                RecognitionApi.CONTINUOUS_PROFILE_DEFAULT, callback,
            )

            override fun recognizeContinuousWithProfile(
                clientPackage: String?,
                sessionId: String?,
                pcmStream: ParcelFileDescriptor?,
                sampleRateHz: Int,
                channelCount: Int,
                bitsPerSample: Int,
                profile: Int,
                callback: IContinuousRecognitionCallback?,
            ) = startContinuousRecognition(
                clientPackage, sessionId, pcmStream, sampleRateHz, channelCount, bitsPerSample, profile, callback,
            )

            override fun getCapabilities(): Int = RecognitionApi.CAPABILITIES
        }

    private fun startOneShotRecognition(
        clientPackage: String?,
        sessionId: String?,
        pcmAudio: ParcelFileDescriptor?,
        sampleRateHz: Int,
        channelCount: Int,
        bitsPerSample: Int,
        callback: IRecognitionCallback?,
        timed: Boolean,
    ) {
        val safeSessionId = sessionId.orEmpty()
        val safeCallback = callback ?: return pcmAudio?.closeQuietly() ?: Unit
        val identity = authenticatedCaller(clientPackage)
        val failure =
            if (identity == null || !authorizations.isAuthorized(identity)) {
                RecognitionApi.ERROR_UNAUTHORIZED to "Доступ не разрешён"
            } else {
                inputFailure(safeSessionId, pcmAudio, sampleRateHz, channelCount, bitsPerSample, "Некорректная аудиозапись")
            }
        if (failure != null) {
            pcmAudio?.closeQuietly()
            return safeCallback.error(safeSessionId, failure.first, failure.second)
        }
        identity!!; pcmAudio!!
        val session = ActiveSession(identity.uid, identity.packageName, safeSessionId, pcmAudio, safeCallback, timed)
        if (!registerExternal(session)) {
            pcmAudio.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_BUSY, "Распознавание уже занято")
            return
        }
        session.start()
    }

    private fun startContinuousRecognition(
        clientPackage: String?,
        sessionId: String?,
        pcmStream: ParcelFileDescriptor?,
        sampleRateHz: Int,
        channelCount: Int,
        bitsPerSample: Int,
        profile: Int,
        callback: IContinuousRecognitionCallback?,
    ) {
        val safeSessionId = sessionId.orEmpty()
        val safeCallback = callback ?: return pcmStream?.closeQuietly() ?: Unit
        val identity = authenticatedCaller(clientPackage)
        val failure =
            if (identity == null || !authorizations.isAuthorized(identity)) {
                RecognitionApi.ERROR_UNAUTHORIZED to "Доступ не разрешён"
            } else {
                inputFailure(safeSessionId, pcmStream, sampleRateHz, channelCount, bitsPerSample, "Некорректный аудиопоток")
            }
        if (failure != null) {
            pcmStream?.closeQuietly()
            return safeCallback.error(safeSessionId, failure.first, failure.second)
        }
        identity!!; pcmStream!!
        val minimumSilenceSeconds =
            ContinuousRecognitionProfile.minimumSilenceSeconds(
                profile = profile,
                defaultSeconds = Prefs.getPauseLengthSeconds(this@RecognitionService),
            )
        val session =
            ContinuousSession(
                identity.uid, identity.packageName, safeSessionId, pcmStream, minimumSilenceSeconds, safeCallback,
                ContinuousRecognitionProfile.wordTimestampsRequested(profile),
                ContinuousRecognitionProfile.dictationSignalRequested(profile),
            )
        if (!registerExternal(session)) {
            pcmStream.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_BUSY, "Распознавание уже занято")
            return
        }
        session.start()
    }

    /** The first reason the input cannot start a session (code and message), or null. */
    private fun inputFailure(
        sessionId: String,
        descriptor: ParcelFileDescriptor?,
        sampleRateHz: Int,
        channelCount: Int,
        bitsPerSample: Int,
        invalidMessage: String,
    ): Pair<Int, String>? =
        when {
            sessionId.isBlank() || descriptor == null -> RecognitionApi.ERROR_INVALID_AUDIO to invalidMessage
            else -> PcmAudioValidator.metadataError(sampleRateHz, channelCount, bitsPerSample)?.let { it to "Поддерживается PCM 16 кГц, mono, 16 bit" }
                ?: if (GigaAmModel.isInstalled(this)) null else RecognitionApi.ERROR_MODEL_UNAVAILABLE to "Модель ещё не готова"
        }

    private fun registerExternal(session: RecognitionProviderSession): Boolean =
        synchronized(sessionLock) {
            if (sessions.any { it.uid == session.uid && it.sessionId == session.sessionId }) return@synchronized false
            val lease =
                RecognitionSessionCoordinator.process.acquireExternal(session.uid, session.onPrimaryChange, session::preempt)
                    ?: return@synchronized false
            session.attach(lease)
            sessions += session
            true
        }

    private fun authenticatedCaller(clientPackage: String?): CallerIdentity? =
        clientPackage?.let { identities.resolve(Binder.getCallingUid(), it) }

    private inner class ActiveSession(
        override val uid: Int,
        override val packageName: String,
        override val sessionId: String,
        private val descriptor: ParcelFileDescriptor,
        private val callback: IRecognitionCallback,
        private val timed: Boolean = false,
    ) : RecognitionProviderSession {
        override var recognitionLease: RecognitionSessionArbiter.Lease? = null
        private val terminal = AtomicBoolean(false)
        private val deathRecipient = IBinder.DeathRecipient { cancel(notify = false) }
        private var job: Job? = null

        fun start() {
            if (terminal.get()) return
            val work =
                scope.launch(start = CoroutineStart.LAZY) {
                    try {
                        val transcript =
                            OneShotRecognitionRunner(this@RecognitionService, descriptor).runTimed(timed)
                        if (transcript.text.isBlank()) {
                            finishError(RecognitionApi.ERROR_RECOGNITION_FAILED, "Речь не распознана")
                        } else {
                            finishResult(transcript)
                        }
                    } catch (_: PcmLimitException) {
                        finishError(RecognitionApi.ERROR_LIMIT_EXCEEDED, "Файл не удалось обработать безопасно")
                    } catch (_: LongFormResultLimitException) {
                        finishError(
                            RecognitionApi.ERROR_LIMIT_EXCEEDED,
                            "Итоговый текст слишком велик; используйте потоковый callback",
                        )
                    } catch (_: InvalidPcmException) {
                        finishError(RecognitionApi.ERROR_INVALID_AUDIO, "Некорректная аудиозапись")
                    } catch (_: CancellationException) {
                        finishError(RecognitionApi.ERROR_CANCELED, "Запрос отменён")
                    } catch (_: Throwable) {
                        finishError(RecognitionApi.ERROR_RECOGNITION_FAILED, "Не удалось распознать речь")
                    }
                }
            job = work
            work.invokeOnCompletion { cleanup() }
            runCatching { callback.asBinder().linkToDeath(deathRecipient, 0) }
                .onFailure {
                    finishError(RecognitionApi.ERROR_CANCELED, "Запрос отменён")
                    work.cancel()
                    return
                }
            work.start()
        }

        override fun cancel(notify: Boolean) {
            job?.cancel()
            descriptor.closeQuietly()
            if (notify) finishError(RecognitionApi.ERROR_CANCELED, "Запрос отменён") else finishSilently()
        }

        override fun cancelWithError(code: Int, message: String) {
            finishError(code, message)
            descriptor.closeQuietly()
            job?.cancel()
        }

        private fun finishResult(transcript: TimedTranscript) {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
            if (timed) {
                runCatching {
                    callback.onFinalResultTimed(
                        sessionId,
                        transcript.text,
                        LongArray(transcript.words.size) { transcript.words[it].startMs },
                        LongArray(transcript.words.size) { transcript.words[it].endMs },
                        Array(transcript.words.size) { transcript.words[it].text },
                    )
                }
            } else {
                runCatching { callback.onFinalResult(sessionId, transcript.text) }
            }
        }

        private fun releaseActiveSlot() {
            synchronized(sessionLock) { sessions.remove(this) }
            RecognitionSessionCoordinator.process.release(recognitionLease)
            recognitionLease = null
        }

        private fun finishError(code: Int, message: String) {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
            runCatching { callback.onError(sessionId, code, message) }
        }

        private fun finishSilently() {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
        }

        private fun cleanup() {
            descriptor.closeQuietly()
            runCatching { callback.asBinder().unlinkToDeath(deathRecipient, 0) }
            releaseActiveSlot()
        }
    }

    private inner class ContinuousSession(
        override val uid: Int,
        override val packageName: String,
        override val sessionId: String,
        private val descriptor: ParcelFileDescriptor,
        private val minimumSilenceSeconds: Float,
        private val callback: IContinuousRecognitionCallback,
        private val wordTimestamps: Boolean = false,
        dictationSignal: Boolean = false,
    ) : RecognitionProviderSession {
        override var recognitionLease: RecognitionSessionArbiter.Lease? = null
        private val terminal = AtomicBoolean(false)
        private val deathRecipient = IBinder.DeathRecipient { cancel(notify = false) }
        private var job: Job? = null
        private val hold =
            if (dictationSignal) {
                DictationHold({ active, at -> runCatching { callback.onPrimaryDictation(sessionId, active, at) } })
            } else {
                null
            }
        override val onPrimaryChange: (() -> Unit)? = hold?.let { it::sync }

        fun start() {
            if (terminal.get()) return
            val work =
                scope.launch(start = CoroutineStart.LAZY) {
                    try {
                        ContinuousRecognitionPipeline(
                            context = this@RecognitionService,
                            descriptor = descriptor,
                            minimumSilenceSeconds = minimumSilenceSeconds,
                            onReady = { runCatching { callback.onReady(sessionId) } },
                            onSpeechState = { active ->
                                runCatching { callback.onSpeechStateChanged(sessionId, active) }
                            },
                            onResult = { sequence, text ->
                                runCatching { callback.onFinalResult(sessionId, sequence, text) }
                            },
                            onTimedResult = if (wordTimestamps) ::deliverTimed else null,
                            hold = hold,
                        ).run()
                        finishStopped()
                    } catch (_: CancellationException) {
                        finishError(RecognitionApi.ERROR_CANCELED, "Запрос отменён")
                    } catch (_: Throwable) {
                        finishError(RecognitionApi.ERROR_RECOGNITION_FAILED, "Не удалось распознать речь")
                    }
                }
            job = work
            work.invokeOnCompletion { cleanup() }
            runCatching { callback.asBinder().linkToDeath(deathRecipient, 0) }
                .onFailure {
                    finishSilently()
                    work.cancel()
                    return
                }
            work.start()
        }

        private fun deliverTimed(
            sequence: Long,
            text: String,
            words: List<TimedWord>,
        ) {
            runCatching {
                callback.onFinalResultTimed(
                    sessionId,
                    sequence,
                    text,
                    LongArray(words.size) { words[it].startMs },
                    LongArray(words.size) { words[it].endMs },
                    Array(words.size) { words[it].text },
                )
            }
        }

        override fun cancel(notify: Boolean) {
            descriptor.closeQuietly()
            job?.cancel()
            if (notify) finishError(RecognitionApi.ERROR_CANCELED, "Запрос отменён") else finishSilently()
        }

        override fun cancelWithError(code: Int, message: String) {
            finishError(code, message)
            descriptor.closeQuietly()
            job?.cancel()
        }

        private fun finishStopped() {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
            runCatching { callback.onStopped(sessionId) }
        }

        private fun finishError(code: Int, message: String) {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
            runCatching { callback.onError(sessionId, code, message) }
        }

        private fun finishSilently() {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
        }

        private fun releaseActiveSlot() {
            synchronized(sessionLock) { sessions.remove(this) }
            RecognitionSessionCoordinator.process.release(recognitionLease)
            recognitionLease = null
        }

        private fun cleanup() {
            descriptor.closeQuietly()
            runCatching { callback.asBinder().unlinkToDeath(deathRecipient, 0) }
            releaseActiveSlot()
        }
    }

    internal fun cancelPackage(packageName: String) {
        synchronized(sessionLock) { sessions.filter { it.packageName == packageName } }
            .forEach { it.cancel(notify = true) }
    }

}
