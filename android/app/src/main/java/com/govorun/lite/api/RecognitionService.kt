package com.govorun.lite.api

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
    private var activeSession: RecognitionProviderSession? = null

    override fun onCreate() {
        super.onCreate()
        RecognitionApiControl.attach(this)
        identities = CallerIdentityResolver(this)
        authorizations = RecognitionAuthorizationStore(this)
    }

    override fun onBind(intent: Intent?): IBinder? =
        binder.takeIf { intent?.action == RecognitionApi.SERVICE_ACTION }

    override fun onDestroy() {
        synchronized(sessionLock) { activeSession?.cancel(notify = false) }
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
            ) {
                val safeSessionId = sessionId.orEmpty()
                val safeCallback = callback ?: run {
                    pcmAudio?.closeQuietly()
                    return
                }
                val identity = authenticatedCaller(clientPackage)
                if (identity == null || !authorizations.isAuthorized(identity)) {
                    pcmAudio?.closeQuietly()
                    safeCallback.error(safeSessionId, RecognitionApi.ERROR_UNAUTHORIZED, "Доступ не разрешён")
                    return
                }
                if (safeSessionId.isBlank() || pcmAudio == null) {
                    pcmAudio?.closeQuietly()
                    safeCallback.error(safeSessionId, RecognitionApi.ERROR_INVALID_AUDIO, "Некорректная аудиозапись")
                    return
                }
                PcmAudioValidator.metadataError(sampleRateHz, channelCount, bitsPerSample)?.let { code ->
                    pcmAudio.closeQuietly()
                    safeCallback.error(safeSessionId, code, "Поддерживается PCM 16 кГц, mono, 16 bit")
                    return
                }
                if (!GigaAmModel.isInstalled(this@RecognitionService)) {
                    pcmAudio.closeQuietly()
                    safeCallback.error(safeSessionId, RecognitionApi.ERROR_MODEL_UNAVAILABLE, "Модель ещё не готова")
                    return
                }

                val session = ActiveSession(identity.uid, identity.packageName, safeSessionId, pcmAudio, safeCallback)
                if (!registerExternal(session)) {
                    pcmAudio.closeQuietly()
                    safeCallback.error(safeSessionId, RecognitionApi.ERROR_BUSY, "Распознавание уже занято")
                    return
                }
                session.start()
            }

            override fun cancel(clientPackage: String?, sessionId: String?) {
                val identity = authenticatedCaller(clientPackage) ?: return
                synchronized(sessionLock) {
                    activeSession
                        ?.takeIf {
                            it.uid == identity.uid &&
                                it.packageName == identity.packageName &&
                                it.sessionId == sessionId
                        }?.cancel(notify = true)
                }
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
            ) {
                startContinuousRecognition(
                    clientPackage = clientPackage,
                    sessionId = sessionId,
                    pcmStream = pcmStream,
                    sampleRateHz = sampleRateHz,
                    channelCount = channelCount,
                    bitsPerSample = bitsPerSample,
                    profile = RecognitionApi.CONTINUOUS_PROFILE_DEFAULT,
                    callback = callback,
                )
            }

            override fun recognizeContinuousWithProfile(
                clientPackage: String?,
                sessionId: String?,
                pcmStream: ParcelFileDescriptor?,
                sampleRateHz: Int,
                channelCount: Int,
                bitsPerSample: Int,
                profile: Int,
                callback: IContinuousRecognitionCallback?,
            ) {
                startContinuousRecognition(
                    clientPackage = clientPackage,
                    sessionId = sessionId,
                    pcmStream = pcmStream,
                    sampleRateHz = sampleRateHz,
                    channelCount = channelCount,
                    bitsPerSample = bitsPerSample,
                    profile = profile,
                    callback = callback,
                )
            }

            override fun getCapabilities(): Int = RecognitionApi.CAPABILITIES
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
        val safeCallback = callback ?: run {
            pcmStream?.closeQuietly()
            return
        }
        val identity = authenticatedCaller(clientPackage)
        if (identity == null || !authorizations.isAuthorized(identity)) {
            pcmStream?.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_UNAUTHORIZED, "Доступ не разрешён")
            return
        }
        if (safeSessionId.isBlank() || pcmStream == null) {
            pcmStream?.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_INVALID_AUDIO, "Некорректный аудиопоток")
            return
        }
        PcmAudioValidator.metadataError(sampleRateHz, channelCount, bitsPerSample)?.let { code ->
            pcmStream.closeQuietly()
            safeCallback.error(safeSessionId, code, "Поддерживается PCM 16 кГц, mono, 16 bit")
            return
        }
        if (!GigaAmModel.isInstalled(this@RecognitionService)) {
            pcmStream.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_MODEL_UNAVAILABLE, "Модель ещё не готова")
            return
        }

        val minimumSilenceSeconds =
            ContinuousRecognitionProfile.minimumSilenceSeconds(
                profile = profile,
                defaultSeconds = Prefs.getPauseLengthSeconds(this@RecognitionService),
            )
        val session =
            ContinuousSession(
                identity.uid,
                identity.packageName,
                safeSessionId,
                pcmStream,
                minimumSilenceSeconds,
                safeCallback,
            )
        if (!registerExternal(session)) {
            pcmStream.closeQuietly()
            safeCallback.error(safeSessionId, RecognitionApi.ERROR_BUSY, "Распознавание уже занято")
            return
        }
        session.start()
    }

    private fun registerExternal(session: RecognitionProviderSession): Boolean =
        synchronized(sessionLock) {
            if (activeSession != null) return@synchronized false
            activeSession = session
            val lease = RecognitionSessionCoordinator.process.acquireExternal(session::preempt)
            if (lease == null) {
                activeSession = null
                false
            } else {
                session.attach(lease)
                true
            }
        }

    private fun authenticatedCaller(clientPackage: String?): CallerIdentity? =
        clientPackage?.let { identities.resolve(Binder.getCallingUid(), it) }

    private inner class ActiveSession(
        override val uid: Int,
        override val packageName: String,
        override val sessionId: String,
        private val descriptor: ParcelFileDescriptor,
        private val callback: IRecognitionCallback,
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
                            OneShotRecognitionRunner(this@RecognitionService, descriptor).run()
                        if (transcript.isBlank()) {
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

        private fun finishResult(text: String) {
            if (!terminal.compareAndSet(false, true)) return
            releaseActiveSlot()
            runCatching { callback.onFinalResult(sessionId, text) }
        }

        private fun releaseActiveSlot() {
            synchronized(sessionLock) {
                if (activeSession === this) activeSession = null
            }
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
            synchronized(sessionLock) {
                if (activeSession === this) activeSession = null
            }
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
        synchronized(sessionLock) {
            activeSession?.takeIf { it.packageName == packageName }?.cancel(notify = true)
        }
    }

}
