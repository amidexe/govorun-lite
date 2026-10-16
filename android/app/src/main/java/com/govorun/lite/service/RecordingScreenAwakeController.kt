package com.govorun.lite.service

import android.annotation.SuppressLint
import android.content.Context
import android.os.PowerManager

/**
 * Keeps the display awake while dictation needs it.
 *
 * FLAG_KEEP_SCREEN_ON remains on the accessibility overlay, but recent Android
 * versions do not consistently honour that flag for overlay-only windows. The
 * original Govorun therefore also held a screen WakeLock. This small fallback
 * restores that behaviour and is released on every recording/service exit.
 */
internal class RecordingScreenAwakeController(context: Context) {
    private val powerManager =
        context.getSystemService(Context.POWER_SERVICE) as PowerManager
    private var wakeLock: PowerManager.WakeLock? = null

    fun setActive(active: Boolean) {
        if (active) acquire() else release()
    }

    @SuppressLint("WakelockTimeout")
    @Suppress("DEPRECATION")
    private fun acquire() {
        val lock = wakeLock ?: powerManager.newWakeLock(
            PowerManager.SCREEN_BRIGHT_WAKE_LOCK,
            "govorun:active-dictation",
        ).apply {
            setReferenceCounted(false)
            wakeLock = this
        }
        if (!lock.isHeld) lock.acquire()
    }

    fun release() {
        wakeLock?.let { lock ->
            if (lock.isHeld) lock.release()
        }
        wakeLock = null
    }
}
