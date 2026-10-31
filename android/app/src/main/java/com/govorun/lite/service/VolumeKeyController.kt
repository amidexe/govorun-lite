package com.govorun.lite.service

import android.app.KeyguardManager
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.KeyEvent
import com.govorun.lite.api.CallerIdentityResolver
import com.govorun.lite.api.RecognitionApi
import com.govorun.lite.api.RecognitionAuthorizationStore
import com.govorun.lite.util.VolumeKeyPrefs

/** Where a held volume-down goes. */
internal enum class VolumeKeyRoute { PASS, DICTATE, APP }

/**
 * Volume-down held (an experiment, both parts off by default). Only Govorun has accessibility; it listens to the key and lends it.
 * With «Диктовка по удержанию «−»» on, where its bubble is shown a hold dictates exactly as holding the bubble does. On the lock
 * screen a hold records in Говорун.Эхо when the person gave it the key in «API для приложений»; that switch alone decides it. A
 * short press, a call, an alarm or music: the key stays an ordinary volume key.
 */
internal object VolumeKeyRules {
    // Long enough to tell a hold from a quick press on the key (~100–150 ms), no longer: half a second felt slow (owner, 25 Oct).
    const val HOLD_MS = 350L

    fun route(
        enabled: Boolean,
        locked: Boolean,
        audioBusy: Boolean,
        bubbleVisible: Boolean,
        appTarget: Boolean,
    ): VolumeKeyRoute =
        when {
            audioBusy -> VolumeKeyRoute.PASS
            locked -> if (appTarget) VolumeKeyRoute.APP else VolumeKeyRoute.PASS
            enabled && bubbleVisible -> VolumeKeyRoute.DICTATE
            else -> VolumeKeyRoute.PASS
        }
}

/** The key as holding the bubble: down prepares the microphone, a hold starts, the release stops, a short press cancels. */
internal fun volumeKeyForBubble(
    context: Context,
    bubble: BubbleOverlayController.Callbacks,
    bubbleVisible: () -> Boolean,
) = VolumeKeyController(context, bubbleVisible, bubble::isVadActive, bubble::onPointerDown, bubble::onPointerCancel, bubble::onHoldStart) {
    bubble.onHoldStop(false)
}

internal class VolumeKeyController(
    private val context: Context,
    private val bubbleVisible: () -> Boolean,
    private val dictationActive: () -> Boolean,
    private val onDictateDown: () -> Unit,
    private val onDictateCancel: () -> Unit,
    private val onDictateStart: () -> Unit,
    private val onDictateStop: () -> Unit,
) {
    private enum class Phase { Idle, Pending, Active, Passthrough }

    private val handler = Handler(Looper.getMainLooper())
    private var phase = Phase.Idle
    private var route = VolumeKeyRoute.PASS
    private var target: Intent? = null
    private val longPress = Runnable(::onLongPress)

    /** True when the event is consumed; anything Govorun does not take reaches the system as an ordinary key. */
    fun onKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode != KeyEvent.KEYCODE_VOLUME_DOWN) return false
        return runCatching {
            when (event.action) {
                KeyEvent.ACTION_DOWN -> onDown(event)
                KeyEvent.ACTION_UP -> onUp()
                else -> false
            }
        }.getOrElse {
            Log.w(TAG, "event=volume_key_failed", it)
            reset()
            false
        }
    }

    private fun onDown(event: KeyEvent): Boolean {
        if (event.repeatCount > 0) return phase == Phase.Pending || phase == Phase.Active
        if (phase != Phase.Idle) return phase != Phase.Passthrough
        val locked = context.getSystemService(KeyguardManager::class.java)?.isKeyguardLocked == true
        target = if (locked) appTarget() else null
        route =
            VolumeKeyRules.route(
                enabled = VolumeKeyPrefs.isEnabled(context),
                locked = locked,
                audioBusy = audioBusy(),
                // A dictation the person started by a tap is not the key's to stop.
                bubbleVisible = bubbleVisible() && !dictationActive(),
                appTarget = target != null,
            )
        if (route == VolumeKeyRoute.PASS) {
            phase = Phase.Passthrough
            return false
        }
        phase = Phase.Pending
        if (route == VolumeKeyRoute.DICTATE) onDictateDown()
        handler.postDelayed(longPress, VolumeKeyRules.HOLD_MS)
        return true
    }

    private fun onLongPress() {
        if (phase != Phase.Pending) return
        phase = Phase.Active
        Log.i(TAG, "event=volume_key_hold route=$route")
        when (route) {
            VolumeKeyRoute.DICTATE -> onDictateStart()
            VolumeKeyRoute.APP -> launch(RecognitionApi.VOLUME_KEY_START)
            VolumeKeyRoute.PASS -> Unit
        }
    }

    private fun onUp(): Boolean {
        val was = phase
        handler.removeCallbacks(longPress)
        phase = Phase.Idle
        return when (was) {
            Phase.Pending -> {
                // A short press: the volume the person meant, as the system would have set it.
                if (route == VolumeKeyRoute.DICTATE) onDictateCancel()
                context.getSystemService(AudioManager::class.java)?.adjustSuggestedStreamVolume(
                    AudioManager.ADJUST_LOWER,
                    AudioManager.USE_DEFAULT_STREAM_TYPE,
                    AudioManager.FLAG_SHOW_UI,
                )
                true
            }
            Phase.Active -> {
                when (route) {
                    VolumeKeyRoute.DICTATE -> onDictateStop()
                    VolumeKeyRoute.APP -> launch(RecognitionApi.VOLUME_KEY_STOP)
                    VolumeKeyRoute.PASS -> Unit
                }
                true
            }
            Phase.Passthrough, Phase.Idle -> false
        }
    }

    fun reset() {
        handler.removeCallbacks(longPress)
        phase = Phase.Idle
    }

    /** A call, a ringing phone, an alarm or music: the person is using the key for the sound. */
    private fun audioBusy(): Boolean {
        val audio = context.getSystemService(AudioManager::class.java) ?: return true
        if (audio.mode != AudioManager.MODE_NORMAL || audio.isMusicActive) return true
        return runCatching {
            audio.activePlaybackConfigurations.any {
                it.audioAttributes.usage == AudioAttributes.USAGE_ALARM ||
                    it.audioAttributes.usage == AudioAttributes.USAGE_NOTIFICATION_RINGTONE
            }
        }.getOrDefault(false)
    }

    /**
     * The one app the person gave the volume key: still installed with the signers it was approved with, offering an activity
     * for [RecognitionApi.VOLUME_KEY_RECORD_ACTION] that only Govorun's signature permission can open.
     */
    private fun appTarget(): Intent? {
        val store = RecognitionAuthorizationStore(context)
        val resolver = CallerIdentityResolver(context)
        val permission = RecognitionApi.volumeKeyPermission(context.packageName)
        for (packageName in store.approvedPackages()) {
            val identity = resolver.resolveInstalled(packageName) ?: continue
            if (!store.hasScope(identity, RecognitionApi.SCOPE_VOLUME_KEY)) continue
            val intent = Intent(RecognitionApi.VOLUME_KEY_RECORD_ACTION).setPackage(packageName)
            val activity = context.packageManager.queryIntentActivities(intent, 0).firstOrNull()?.activityInfo ?: continue
            if (activity.permission != permission) continue
            return intent.setClassName(activity.packageName, activity.name)
        }
        return null
    }

    private fun launch(event: String) {
        val intent = target ?: return
        runCatching {
            context.startActivity(
                Intent(intent)
                    .putExtra(RecognitionApi.EXTRA_VOLUME_KEY_EVENT, event)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_NO_ANIMATION),
            )
        }.onFailure { Log.w(TAG, "event=volume_key_app_failed", it) }
    }

    private companion object {
        const val TAG = "VolumeKey"
    }
}
