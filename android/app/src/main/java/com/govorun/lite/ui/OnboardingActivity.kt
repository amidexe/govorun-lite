package com.govorun.lite.ui

import android.Manifest
import android.annotation.SuppressLint
import android.content.ActivityNotFoundException
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.os.PowerManager
import android.provider.Settings
import androidx.activity.OnBackPressedCallback
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.model.GigaAmModel
import com.govorun.lite.stats.AndroidStatsRecorder
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.storage.GovorunPreferences
import com.govorun.lite.transcriber.OfflineTranscriber
import com.govorun.lite.transcriber.VadRecorder
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.onboarding.OnboardingScreen
import com.govorun.lite.ui.onboarding.OnboardingState
import com.govorun.lite.ui.onboarding.OnboardingStep
import com.govorun.lite.util.StorageKeys
import com.govorun.lite.util.RecordingModePrefs
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class OnboardingActivity : AppCompatActivity() {
    private val preferences by lazy { GovorunPreferences.of(this, PREFS_NAME) }
    private var state = androidx.compose.runtime.mutableStateOf(OnboardingState())
    private var recorder: VadRecorder? = null
    private var recordStartMs = 0L

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        update { copy(micGranted = granted, micDenied = !granted) }
        if (granted) startRecording()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val recovery = intent.getBooleanExtra(EXTRA_RECOVERY, false)
        if (preferences.getBoolean(KEY_ONBOARDING_DONE, false) && !recovery) {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
            return
        }
        val step = if (recovery) recoveryStep() else runCatching {
            parseOnboardingStep(preferences.getString(KEY_STEP, null))
        }.getOrDefault(OnboardingStep.WELCOME)
        state.value = readState(step).copy(
            recordingMode = savedInstanceState?.getString(STATE_RECORDING_MODE)
                ?: RecordingModePrefs.onboardingChoice(this),
        )
        setContent {
            GovorunTheme {
                OnboardingScreen(
                    state = state.value,
                    onBack = ::previousStep,
                    onContinue = ::nextStep,
                    onTry = ::tryMicrophone,
                    onAppSettings = ::openAppSettings,
                    onAccessibility = {
                        com.govorun.lite.util.AccessibilityHelper
                            .openAccessibilitySettings(this)
                    },
                    onBattery = ::requestBattery,
                    onRecordingMode = { mode -> update { copy(recordingMode = mode) } },
                )
            }
        }
        watchModel()
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (state.value.step == OnboardingStep.WELCOME) {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                } else {
                    previousStep()
                }
            }
        })
    }

    override fun onResume() {
        super.onResume()
        update {
            copy(
                micGranted = hasMicPermission(),
                accessibilityReady = accessibilityReady(),
                batteryReady = isBatteryAllowed(),
            )
        }
    }

    private fun readState(step: OnboardingStep): OnboardingState {
        return OnboardingState(
            step = step,
            micGranted = hasMicPermission(),
            modelReady = GigaAmModel.isInstalled(this),
            accessibilityReady = accessibilityReady(),
            batteryReady = isBatteryAllowed(),
            showModeChoice = !preferences.getBoolean(KEY_ONBOARDING_DONE, false),
        )
    }

    private fun nextStep() {
        if (!state.value.canContinue) return
        val entries = state.value.steps
        val index = entries.indexOf(state.value.step)
        if (index == entries.lastIndex) {
            finishOnboarding()
            return
        }
        if (state.value.recording) stopRecording()
        val next = entries[index + 1]
        preferences.edit().putString(KEY_STEP, next.name).apply()
        update { copy(step = next) }
    }

    private fun previousStep() {
        val entries = state.value.steps
        val index = entries.indexOf(state.value.step)
        if (index <= 0) return
        if (state.value.recording) stopRecording()
        val previous = entries[index - 1]
        preferences.edit().putString(KEY_STEP, previous.name).apply()
        update { copy(step = previous) }
    }

    private fun finishOnboarding() {
        if (state.value.showModeChoice) RecordingModePrefs.write(this, state.value.recordingMode)
        preferences.edit()
            .putBoolean(KEY_ONBOARDING_DONE, true)
            .remove(KEY_STEP)
            .apply()
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_JUST_FINISHED, true),
        )
        finish()
    }

    private fun tryMicrophone() {
        when {
            !hasMicPermission() -> permissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
            state.value.recording -> stopRecording()
            else -> startRecording()
        }
    }

    @SuppressLint("MissingPermission")
    private fun startRecording() {
        if (state.value.recording || !state.value.modelReady) return
        update { copy(recording = true, transcript = "") }
        recordStartMs = android.os.SystemClock.elapsedRealtime()
        val appContext = applicationContext
        recorder = VadRecorder(appContext).also { active ->
            active.start(
                scope = lifecycleScope,
                transcriberProvider = {
                    withContext(Dispatchers.IO) {
                        OfflineTranscriber.getInstance(appContext)
                    }
                },
                onSegment = ::appendSegment,
            )
        }
    }

    private fun stopRecording() {
        if (!state.value.recording) return
        recorder?.stop()
        recorder = null
        update { copy(recording = false) }
        if (recordStartMs > 0) {
            AndroidStatsRecorder.create(applicationContext).addSeconds((android.os.SystemClock.elapsedRealtime() - recordStartMs) / 1000L,
            )
            recordStartMs = 0
        }
    }

    private suspend fun appendSegment(text: String) {
        if (text.isBlank()) return
        withContext(Dispatchers.Main) {
            update {
                copy(transcript = listOf(transcript, text.trim()).filter(String::isNotBlank).joinToString(" "))
            }
            AndroidStatsRecorder.create(applicationContext).addWords(StatsStore.countWords(text))
        }
    }

    private fun watchModel() {
        lifecycleScope.launch {
            while (!GigaAmModel.isInstalled(this@OnboardingActivity)) delay(500)
            update { copy(modelReady = true) }
        }
    }

    private fun requestBattery() {
        val action = if (isBatteryAllowed()) {
            Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS
        } else {
            Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS
        }
        val intent = Intent(action).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (action == Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS) {
            intent.data = Uri.parse("package:$packageName")
        }
        try {
            startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            startActivity(
                Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        }
    }

    private fun openAppSettings() {
        startActivity(
            Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", packageName, null),
            ),
        )
    }

    private fun hasMicPermission() = ContextCompat.checkSelfPermission(
        this,
        Manifest.permission.RECORD_AUDIO,
    ) == PackageManager.PERMISSION_GRANTED

    private fun isBatteryAllowed() =
        (getSystemService(POWER_SERVICE) as? PowerManager)
            ?.isIgnoringBatteryOptimizations(packageName) == true

    private fun accessibilityReady(): Boolean =
        com.govorun.lite.util.AccessibilityHelper.isLiteServiceOperational(this) &&
            !com.govorun.lite.util.AccessibilityHelper.isLiteShortcutEnabled(this)

    private fun recoveryStep(): OnboardingStep = when {
        !hasMicPermission() -> OnboardingStep.TRY_IT
        !accessibilityReady() -> OnboardingStep.ACCESSIBILITY
        else -> OnboardingStep.BATTERY
    }

    private fun update(block: OnboardingState.() -> OnboardingState) {
        state.value = state.value.block()
    }

    private fun parseOnboardingStep(value: String?): OnboardingStep = when (value) {
        "ONLINE_BETA", "ONLINE_STYLIZATION" -> OnboardingStep.BATTERY
        else -> OnboardingStep.valueOf(value.orEmpty())
    }

    override fun onPause() {
        if (state.value.recording) stopRecording()
        super.onPause()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putString(STATE_RECORDING_MODE, state.value.recordingMode)
        super.onSaveInstanceState(outState)
    }

    companion object {
        const val EXTRA_RECOVERY = "recover_core_setup"
        private const val STATE_RECORDING_MODE = "onboarding_recording_mode"
        private const val PREFS_NAME = StorageKeys.PREFS_FILE
        private const val KEY_STEP = StorageKeys.Key.ONBOARDING_STEP
        private const val KEY_ONBOARDING_DONE = StorageKeys.Key.ONBOARDING_DONE
    }
}
