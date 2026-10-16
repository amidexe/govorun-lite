package com.govorun.lite.ui

import android.Manifest
import android.annotation.SuppressLint
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.WindowManager
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.shared.SharedDictionary
import com.govorun.lite.transcriber.DictionaryRuleEditor
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.util.AppLog
import com.govorun.lite.util.HomePreferences
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class DictionaryActivity : ComponentActivity() {
    private val voiceCaptureState = mutableStateOf(DictionaryVoiceCaptureState())
    private var voiceRecorder: VadRecorder? = null
    private var voiceSession = 0L
    private var recordAfterPermission = false
    private var voiceSpeechActive = false
    private val screenAwakeHandler = Handler(Looper.getMainLooper())
    private val clearScreenAwake = Runnable {
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        val shouldStart = recordAfterPermission && voiceCaptureState.value.visible
        recordAfterPermission = false
        if (granted && shouldStart) {
            startVoiceCapture()
        } else if (!granted && voiceCaptureState.value.visible) {
            updateVoiceState { copy(error = getString(R.string.dict_voice_permission_denied)) }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val saved = Prefs.getDictionary(this)
        val initial = if (saved.isBlank()) {
            getString(R.string.dict_starter_template)
        } else {
            saved
        }
        setContent {
            GovorunTheme {
                DictionaryScreen(
                    initialText = initial,
                    initialSavedText = saved,
                    initialEnabled = Prefs.isDictionaryEnabled(this),
                    initialShowOnHome = HomePreferences.showDictionaryOnHome(this),
                    voiceState = voiceCaptureState.value,
                    onBack = ::finish,
                    onEnabled = {
                        GovorunAccountRepository.recordSyncedContentChange(this)
                        Prefs.setDictionaryEnabled(this, it)
                        AppLog.log(this, "Dictionary enabled=$it")
                    },
                    onShowOnHome = {
                        HomePreferences.setShowDictionaryOnHome(this, it)
                        AppLog.log(this, "Dictionary home shortcut visible=$it")
                    },
                    onSave = ::saveAndClose,
                    onSaveVoice = { saveDictionary(it) },
                    onOpenVoice = ::openVoiceCapture,
                    onVoiceCorrectText = ::setVoiceCorrectText,
                    onVoiceRecord = ::requestVoiceCapture,
                    onVoiceStop = ::stopVoiceCapture,
                    onVoiceRemove = ::removeVoiceVariant,
                    onVoiceDismiss = ::dismissVoiceCapture,
                )
            }
        }
    }

    private fun saveAndClose(value: String) {
        if (saveDictionary(value)) finish()
    }

    private fun saveDictionary(value: String): Boolean {
        try { SharedDictionary.validate(value) }
        catch (_: Exception) {
            Toast.makeText(this, R.string.dict_invalid, Toast.LENGTH_LONG).show()
            return false
        }
        GovorunAccountRepository.recordSyncedContentChange(this)
        Prefs.setDictionary(this, value)
        Toast.makeText(this, R.string.dict_saved, Toast.LENGTH_SHORT).show()
        AppLog.log(this, "Dictionary saved")
        return true
    }

    private fun openVoiceCapture() {
        if (!voiceCaptureState.value.visible) {
            voiceCaptureState.value = DictionaryVoiceCaptureState(visible = true)
        }
    }

    private fun setVoiceCorrectText(value: String) {
        updateVoiceState {
            if (busy) this else copy(correctText = value, error = null)
        }
    }

    private fun requestVoiceCapture() {
        if (voiceCaptureState.value.correctText.isBlank()) {
            updateVoiceState { copy(error = getString(R.string.dict_voice_enter_correct)) }
            return
        }
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            recordAfterPermission = true
            permissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
            return
        }
        startVoiceCapture()
    }

    @SuppressLint("MissingPermission")
    private fun startVoiceCapture() {
        if (!voiceCaptureState.value.visible || voiceCaptureState.value.busy) return
        if (!GigaAmModel.isInstalled(this)) {
            updateVoiceState { copy(error = getString(R.string.dict_voice_model_unavailable)) }
            return
        }
        val session = ++voiceSession
        val appContext = applicationContext
        val active = VadRecorder(appContext)
        voiceRecorder = active
        updateVoiceState { copy(recording = true, processing = false, error = null) }
        updateVoiceScreenAwake()
        active.start(
            scope = lifecycleScope,
            transcriberProvider = {
                withContext(Dispatchers.IO) { OfflineTranscriber.getInstance(appContext) }
            },
            onSegment = { addVoiceVariant(session, it) },
            onDone = { hadText -> finishVoiceCapture(session, hadText) },
            onSpeechActive = { updateVoiceScreenAwake(speechActive = it, allowGrace = !it) },
            minSilenceSeconds = Prefs.PAUSE_SHORT_SECONDS,
        )
        if (!active.isActive) {
            voiceRecorder = null
            updateVoiceState {
                copy(
                    recording = false,
                    processing = false,
                    error = getString(R.string.dict_voice_microphone_busy),
                )
            }
            updateVoiceScreenAwake()
        }
    }

    private fun stopVoiceCapture() {
        val active = voiceRecorder ?: return
        if (!voiceCaptureState.value.recording) return
        updateVoiceState { copy(recording = false, processing = true) }
        updateVoiceScreenAwake()
        active.stop()
    }

    private fun addVoiceVariant(session: Long, raw: String) {
        if (session != voiceSession) return
        updateVoiceState {
            val updated = runCatching {
                DictionaryRuleEditor.appendUniqueVoiceVariants(variants, raw, correctText)
            }.getOrElse { return@updateVoiceState copy(error = getString(R.string.dict_invalid)) }
            if (updated == variants) {
                this
            } else {
                copy(variants = updated, error = null)
            }
        }
    }

    private fun finishVoiceCapture(session: Long, hadText: Boolean) {
        if (session != voiceSession) return
        voiceRecorder = null
        updateVoiceState {
            copy(
                recording = false,
                processing = false,
                error = if (!hadText && variants.isEmpty()) {
                    getString(R.string.dict_voice_not_recognized)
                } else {
                    null
                },
            )
        }
        updateVoiceScreenAwake()
    }

    private fun removeVoiceVariant(index: Int) {
        updateVoiceState {
            if (busy || index !in variants.indices) this else copy(variants = variants.toMutableList().also { it.removeAt(index) })
        }
    }

    private fun dismissVoiceCapture() {
        recordAfterPermission = false
        ++voiceSession
        voiceRecorder?.cancel()
        voiceRecorder = null
        voiceCaptureState.value = DictionaryVoiceCaptureState()
        updateVoiceScreenAwake()
    }

    private fun updateVoiceState(block: DictionaryVoiceCaptureState.() -> DictionaryVoiceCaptureState) {
        voiceCaptureState.value = voiceCaptureState.value.block()
    }

    private fun updateVoiceScreenAwake(
        speechActive: Boolean? = null,
        allowGrace: Boolean = false,
    ) {
        if (speechActive != null) voiceSpeechActive = speechActive
        val state = voiceCaptureState.value
        val shouldKeepAwake = DictionaryVoiceAwakePolicy.shouldKeepAwake(
            recording = state.recording,
            processing = state.processing,
            speechActive = voiceSpeechActive,
            keepForWholeRecording = Prefs.isKeepScreenOnEnabled(this),
        )
        screenAwakeHandler.removeCallbacks(clearScreenAwake)
        when {
            shouldKeepAwake -> window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            allowGrace && state.recording -> screenAwakeHandler.postDelayed(clearScreenAwake, SCREEN_GRACE_MS)
            else -> clearScreenAwake.run()
        }
        if (!state.recording) voiceSpeechActive = false
    }

    override fun onPause() {
        if (voiceRecorder != null) {
            ++voiceSession
            voiceRecorder?.cancel()
            voiceRecorder = null
            updateVoiceState { copy(recording = false, processing = false) }
        }
        updateVoiceScreenAwake()
        super.onPause()
    }

    override fun onDestroy() {
        screenAwakeHandler.removeCallbacks(clearScreenAwake)
        clearScreenAwake.run()
        super.onDestroy()
    }

    private companion object {
        const val SCREEN_GRACE_MS = 5_000L
    }
}
