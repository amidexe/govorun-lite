package com.govorun.lite.transcriber

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import androidx.core.content.ContextCompat
import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean

/** One microphone and FIFO, handed from pointer admission to recognition. */
class MicrophoneCapture internal constructor(
    internal val capture: BufferedPcmCapture,
    private val reading: AtomicBoolean,
    private val stopDevice: () -> Unit,
    private val releaseDevice: () -> Unit,
) {
    private val closed = AtomicBoolean(false)
    internal fun stop() {
        if (reading.getAndSet(false)) try { stopDevice() } catch (_: Exception) {}
    }
    internal suspend fun close() {
        if (!closed.compareAndSet(false, true)) return
        stop()
        capture.job.cancel()
        withContext(NonCancellable) { capture.job.join(); capture.frames.cancel(); releaseDevice() }
    }
    internal fun discard(scope: CoroutineScope) {
        stop()
        capture.job.cancel()
        capture.frames.cancel()
        scope.launch(NonCancellable + Dispatchers.IO) { close() }
    }
    companion object {
        @SuppressLint("MissingPermission")
        internal fun open(context: Context, scope: CoroutineScope): MicrophoneCapture? {
            if (!scope.isActive) return null
            if (ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO)
                != PackageManager.PERMISSION_GRANTED) return null
            val bytes = VadRecorder.VAD_WINDOW_SIZE * 2
            val device = try {
                AudioRecord(MediaRecorder.AudioSource.MIC, VadRecorder.SAMPLE_RATE,
                    AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT,
                    AudioRecord.getMinBufferSize(VadRecorder.SAMPLE_RATE,
                        AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT).coerceAtLeast(bytes * 8))
            } catch (_: Exception) { return null }
            try {
                if (device.state != AudioRecord.STATE_INITIALIZED) { device.release(); return null }
                device.startRecording()
            } catch (_: Exception) { device.release(); return null }
            val reading = AtomicBoolean(true)
            return MicrophoneCapture(BufferedPcmCapture(scope, bytes, reading::get, device::read),
                reading, device::stop, device::release)
        }
    }
}

/** Main-thread gesture handoff; cancellation never creates a recognition job. */
class PendingMicrophoneCapture internal constructor(private val open: () -> MicrophoneCapture?) {
    constructor(context: Context, scope: CoroutineScope) : this({ MicrophoneCapture.open(context, scope) })
    private var pending: MicrophoneCapture? = null
    fun prepare() { if (pending == null) pending = open() }
    fun take(): MicrophoneCapture? = pending.also { pending = null }
    fun discard(scope: CoroutineScope) { take()?.discard(scope) }
}
