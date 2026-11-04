package com.govorun.lite.transcriber

import android.content.Context
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.media.AudioRecord
import android.os.Handler
import android.os.Looper
import android.os.SystemClock

/** Owns only this recording's preference/request. Never changes mode, volume or audio focus. */
internal class RecordingMicrophone(context: Context, headset: Boolean) : AutoCloseable {
    private val audio = context.getSystemService(AudioManager::class.java)
    private data class Candidate(val device: MicrophoneDevice, val input: AudioDeviceInfo?, val communication: AudioDeviceInfo? = null)
    private val communication = MicrophoneCommunication<AudioDeviceInfo>(
        { runCatching { audio.setCommunicationDevice(it) }.getOrDefault(false) },
        { runCatching { audio.clearCommunicationDevice() }; Unit },
    )
    @Volatile private var available = enumerate()
    private val choice = MicrophoneChoice(headset, available.map { it.device })
    @Volatile private var target = choice.current(available.map { it.device })
    private var active: Candidate? = null
    @Volatile private var revision = 0
    private var activeRevision = 0
    @Volatile private var closed = false
    private val devices = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(added: Array<out AudioDeviceInfo>) = refresh(added)
        override fun onAudioDevicesRemoved(removed: Array<out AudioDeviceInfo>) {
            synchronized(this@RecordingMicrophone) {
                // A SCO source can disappear before its sink. Invalidate that recorder/request too.
                if (removed.any { it.id == active?.input?.id || it.id == target?.id }) {
                    revision++
                    communication.close()
                    target?.let(choice::retry)
                }
                refresh()
            }
        }
    }
    private val modes = AudioManager.OnModeChangedListener { refresh() }
    private val routes = AudioManager.OnCommunicationDeviceChangedListener { device ->
        synchronized(this) {
            if (closed || device?.id != audio.communicationDevice?.id) return@OnCommunicationDeviceChangedListener
            // Only an external recovered route re-arms a refused device. Our own set/clear callbacks must not loop.
            if (!communication.owned && device != null && audio.communicationDevice?.id == device.id) {
                choice.retry(MicrophoneDevice(device.id, device.type))
            }
            if (target?.type in BLUETOOTH && device?.id != target?.id) revision++
            refresh()
        }
    }

    init {
        audio.registerAudioDeviceCallback(devices, Handler(Looper.getMainLooper()))
        audio.addOnModeChangedListener(context.mainExecutor, modes)
        audio.addOnCommunicationDeviceChangedListener(context.mainExecutor, routes)
    }

    private fun enumerate(): List<Candidate> {
        val inputs = audio.getDevices(AudioManager.GET_DEVICES_INPUTS).toList()
        val sinks = if (audio.mode == AudioManager.MODE_NORMAL) audio.availableCommunicationDevices else emptyList()
        return inputs.filter { it.type !in BLUETOOTH && MicrophoneChoice.usable(it.type, audio.mode) }
            .map { Candidate(MicrophoneDevice(it.id, it.type), it) } +
            sinks.filter { it.type in BLUETOOTH }.map { sink ->
                Candidate(MicrophoneDevice(sink.id, sink.type), inputs.firstOrNull { matches(it, sink) }, sink)
            }
    }

    @Synchronized private fun refresh(added: Array<out AudioDeviceInfo> = emptyArray()) {
        if (closed) return
        available = enumerate()
        if (!communication.owned) available.filter { candidate ->
            candidate.communication?.let { sink -> added.any { matches(it, sink) } } == true
        }.forEach { choice.retry(it.device) }
        val next = choice.current(available.map { it.device })
        if (target != next || audio.mode != AudioManager.MODE_NORMAL) communication.close()
        target = next
    }

    @Synchronized private fun fallback() {
        communication.close()
        target = choice.fallback(available.map { it.device })
    }

    /** A planned handoff is not a route failure and must not blacklist the newly connected headset. */
    @Synchronized fun recover() {
        if (!closed && revision == activeRevision) {
            val next = choice.recover(active?.device, available.map { it.device })
            if (target != next) communication.close()
            target = next
        }
    }

    /** Capture worker only. Bounded waits; a refused device is retried on connection/route recovery. */
    fun open(bufferBytes: Int, cancelled: () -> Boolean): AudioRecord? {
        var attempts = 0
        while (!closed && !cancelled()) {
            val candidate = synchronized(this) { refresh(); available.firstOrNull { it.device == target } } ?: return null
            val selected = candidate.device
            if (!selected.phone && attempts++ >= 2) {
                synchronized(this) { if (target == selected) fallback() }; continue
            }
            val version = synchronized(this) { revision }
            fun waiting(ms: Long, ready: () -> Boolean): Boolean {
                val deadline = SystemClock.elapsedRealtime() + ms
                while (!closed && !cancelled() && target == selected && version == revision) {
                    if (ready()) return true
                    if (SystemClock.elapsedRealtime() >= deadline) break
                    Thread.sleep(25)
                }
                return false
            }
            var input = candidate.input
            val sink = candidate.communication
            val requested = sink == null || synchronized(this) {
                !closed && audio.mode == AudioManager.MODE_NORMAL && communication.request(sink)
            }
            val connected = requested && (sink == null || waiting(5_000) {
                input = audio.getDevices(AudioManager.GET_DEVICES_INPUTS).firstOrNull { matches(it, sink) }
                audio.communicationDevice?.id == sink.id && input != null
            })
            val record = if (connected) input?.let { MicrophoneCapture.openDevice(bufferBytes, it) } else null
            if (record != null && waiting(1_500) { routedTo(record, input!!) }) {
                // Discard startup audio from any transient system route before delivering a frame.
                val discard = ByteArray(bufferBytes)
                var remaining = bufferBytes * 8
                while (remaining > 0) {
                    val read = record.read(discard, 0, discard.size, AudioRecord.READ_NON_BLOCKING)
                    if (read <= 0) break
                    remaining -= read
                }
                synchronized(this) { active = candidate.copy(input = input); activeRevision = version }
                return record
            }
            record?.let { runCatching { it.stop() }; it.release() }
            if (closed || cancelled()) return null
            synchronized(this) {
                if (target == selected && revision == version) {
                    if (selected.phone) return null
                    fallback()
                }
            }
        }
        return null
    }

    /** A submitted communication request is not proof of the actual AudioRecord input. */
    @Synchronized fun accepts(record: AudioRecord): Boolean = !closed && target == active?.device &&
        revision == activeRevision && active?.input?.let { routedTo(record, it) } == true

    @Synchronized override fun close() {
        if (closed) return
        closed = true
        communication.close()
        audio.unregisterAudioDeviceCallback(devices)
        audio.removeOnModeChangedListener(modes)
        audio.removeOnCommunicationDeviceChangedListener(routes)
    }

    companion object {
        private val BLUETOOTH = setOf(AudioDeviceInfo.TYPE_BLUETOOTH_SCO, AudioDeviceInfo.TYPE_BLE_HEADSET)
        private fun matches(input: AudioDeviceInfo, output: AudioDeviceInfo): Boolean =
            input.type == output.type && (input.address == output.address || input.address.isEmpty() || output.address.isEmpty())
        private fun routedTo(record: AudioRecord, input: AudioDeviceInfo): Boolean =
            runCatching { record.routedDevice?.let { it.id == input.id || (input.type == AudioDeviceInfo.TYPE_BUILTIN_MIC && it.type == input.type) } == true }.getOrDefault(false)
    }
}
