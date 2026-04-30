package com.govorun.lite.util

import android.content.Context
import android.media.AudioManager
import android.media.AudioRecordingConfiguration
import android.util.Log

/**
 * Watches the system's active-recording list and triggers
 * [onForeignRecordingStart] when a recording client other than ours
 * becomes active. The callback fires on the main looper.
 *
 * Used by the accessibility service to interrupt an in-progress
 * dictation session when:
 *  - the user starts a phone call (telephony grabs mic),
 *  - the user holds the mic button in Telegram/WhatsApp/etc to send
 *    a voice message,
 *  - a recorder app or another voice-input app starts capturing.
 *
 * Continuing to capture mic in any of those situations is wrong on
 * two levels:
 *  1) Android routes the live mic stream to the higher-priority client,
 *     so we'd be transcribing silence or filtered garbage anyway and
 *     might still inflate the stats counter from VAD-detected nothing.
 *  2) The user is speaking *to someone else*. Transcribing that
 *     conversation as text into a chat field is semantically wrong and
 *     a real privacy concern (exactly the reason Android's own dialer
 *     blocks third-party recording during calls).
 *
 * Detection works via [AudioManager.AudioRecordingCallback] — registered
 * once per session, filtered by [AudioRecordingConfiguration.getClientAudioSessionId]
 * to ignore our own session. Doesn't require any extra permission.
 */
class MicConflictDetector(
    context: Context,
    private val onForeignRecordingStart: () -> Unit
) {

    companion object {
        private const val TAG = "MicConflict"
    }

    private val audioManager = context.applicationContext
        .getSystemService(Context.AUDIO_SERVICE) as AudioManager

    @Volatile private var ourSessionId: Int = -1
    @Volatile private var registered = false

    private val callback = object : AudioManager.AudioRecordingCallback() {
        override fun onRecordingConfigChanged(configs: MutableList<AudioRecordingConfiguration>) {
            if (ourSessionId < 0) return
            val foreignActive = configs.any { it.clientAudioSessionId != ourSessionId }
            if (foreignActive) {
                Log.i(TAG, "Foreign mic client detected; interrupting our session")
                onForeignRecordingStart()
            }
        }
    }

    /**
     * Begin watching. [ourAudioSessionId] is the session ID returned by
     * [android.media.AudioRecord.getAudioSessionId] for the recorder we
     * just started. Anything in the system recording list that doesn't
     * match this ID is treated as foreign.
     *
     * Also performs an immediate sync check: if a foreign client was
     * already recording when we tried to start, the callback is fired
     * synchronously so the service can stop us before we do useful
     * work over a contended mic.
     */
    fun start(ourAudioSessionId: Int) {
        if (ourAudioSessionId < 0) return
        ourSessionId = ourAudioSessionId
        if (!registered) {
            // Null Handler -> callback dispatched on the calling thread's
            // looper. We're called from the service's main thread, so this
            // means onForeignRecordingStart runs on main — which is what
            // stopVadRecording expects.
            audioManager.registerAudioRecordingCallback(callback, null)
            registered = true
        }
        val existing = try {
            audioManager.activeRecordingConfigurations
        } catch (e: Exception) {
            Log.w(TAG, "activeRecordingConfigurations failed: ${e.message}")
            null
        }
        if (existing != null && existing.any { it.clientAudioSessionId != ourSessionId }) {
            Log.i(TAG, "Foreign mic client already active at our start; interrupting")
            onForeignRecordingStart()
        }
    }

    fun stop() {
        if (registered) {
            try {
                audioManager.unregisterAudioRecordingCallback(callback)
            } catch (e: Exception) {
                Log.w(TAG, "unregisterAudioRecordingCallback failed: ${e.message}")
            }
            registered = false
        }
        ourSessionId = -1
    }
}
