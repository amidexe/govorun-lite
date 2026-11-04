package com.govorun.lite.transcriber

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioDeviceInfo
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
            val reading = AtomicBoolean(true)
            val route = try { RecordingMicrophone(context, com.govorun.lite.util.MicrophonePrefs.headset(context)) }
                catch (_: RuntimeException) { return null }
            val reader = MicrophoneReader(
                open = { route.open(bytes) { !reading.get() || !scope.isActive } },
                accepts = route::accepts, fallback = route::recover,
                readDevice = { device, target, offset, count -> device.read(target, offset, count) },
                stopDevice = { it.stop() }, releaseDevice = { it.release() },
            )
            return MicrophoneCapture(BufferedPcmCapture(scope, bytes, reading::get, reader::read),
                reading, reader::stop, { try { reader.close() } finally { route.close() } })
        }

        @SuppressLint("MissingPermission")
        internal fun openDevice(bytes: Int, input: AudioDeviceInfo): AudioRecord? {
            val minimum = AudioRecord.getMinBufferSize(VadRecorder.SAMPLE_RATE,
                AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
            if (minimum <= 0) return null
            val device = try {
                AudioRecord(MediaRecorder.AudioSource.MIC, VadRecorder.SAMPLE_RATE,
                    AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT,
                    minimum.coerceAtLeast(bytes * 8))
            } catch (_: RuntimeException) { return null }
            try {
                if (device.state == AudioRecord.STATE_INITIALIZED && device.setPreferredDevice(input)) {
                    device.startRecording()
                    if (device.recordingState == AudioRecord.RECORDSTATE_RECORDING) return device
                }
            } catch (_: RuntimeException) { /* Device disappeared or Android refused it. */ }
            device.release()
            return null
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
