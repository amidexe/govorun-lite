package com.govorun.lite.transcriber

import android.media.AudioDeviceInfo
import android.media.AudioManager

/** A session's stable device identity; a Bluetooth communication sink can represent its matching input. */
internal data class MicrophoneDevice(val id: Int, val type: Int) {
    val phone: Boolean get() = type == AudioDeviceInfo.TYPE_BUILTIN_MIC
}

/** Keep a working headset; absence is temporary, while an explicit phone choice lasts the session. */
internal class MicrophoneChoice(private val headset: Boolean, available: List<MicrophoneDevice>) {
    private var selected: MicrophoneDevice? = null
    private val failed = mutableSetOf<MicrophoneDevice>()
    init { current(available) }

    @Synchronized
    fun current(available: List<MicrophoneDevice>): MicrophoneDevice? {
        failed.retainAll(available.toSet()) // A new connection is a new opportunity, even with the same device id.
        val candidates = available.filter { priority(it.type) != null && (headset || it.phone) && it !in failed }
        if (selected !in candidates || selected?.phone != false) selected = candidates
            .minWithOrNull(compareBy({ priority(it.type) }, { it.id }))
        return selected
    }

    @Synchronized
    fun fallback(available: List<MicrophoneDevice>): MicrophoneDevice? {
        selected?.takeUnless { it.phone }?.let { failed += it }
        selected = null
        return current(available)
    }

    @Synchronized fun retry(device: MicrophoneDevice) { failed.remove(device) }

    @Synchronized fun recover(active: MicrophoneDevice?, available: List<MicrophoneDevice>): MicrophoneDevice? =
        current(available).let { if (it == active) fallback(available) else it }

    companion object {
        fun usable(type: Int, mode: Int): Boolean = priority(type) != null &&
            (mode == AudioManager.MODE_NORMAL || type !in setOf(AudioDeviceInfo.TYPE_BLE_HEADSET, AudioDeviceInfo.TYPE_BLUETOOTH_SCO))

        fun priority(type: Int): Int? = when (type) {
            AudioDeviceInfo.TYPE_USB_HEADSET -> 0
            AudioDeviceInfo.TYPE_USB_DEVICE -> 1
            AudioDeviceInfo.TYPE_WIRED_HEADSET -> 2
            AudioDeviceInfo.TYPE_BLE_HEADSET -> 3
            AudioDeviceInfo.TYPE_BLUETOOTH_SCO -> 4
            AudioDeviceInfo.TYPE_BUILTIN_MIC -> 5
            else -> null // A2DP, headphones without a mic, hearing aids and unrelated inputs are not microphones we select.
        }
    }
}

/** Android drops requests on disconnection: every open must submit again, regardless of the reported sink. */
internal class MicrophoneCommunication<T>(private val set: (T) -> Boolean, private val clear: () -> Unit) {
    var owned = false
        private set
    @Synchronized fun request(device: T): Boolean = set(device).also { if (it) owned = true }
    @Synchronized fun close() { if (owned) { owned = false; clear() } }
}
