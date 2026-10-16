package com.govorun.lite.ui.onboarding

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CheckCircle
import com.adamglin.phosphoricons.regular.XCircle
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.RecordingModePrefs
import com.product.ui.ProductWizardShell

data class OnboardingState(
    val step: OnboardingStep = OnboardingStep.WELCOME,
    val micGranted: Boolean = false,
    val micDenied: Boolean = false,
    val modelReady: Boolean = false,
    val recording: Boolean = false,
    val transcript: String = "",
    val accessibilityReady: Boolean = false,
    val batteryReady: Boolean = false,
    val showModeChoice: Boolean = true,
    val recordingMode: String = RecordingModePrefs.REPEAT_TAP,
) {
    val steps: List<OnboardingStep>
        get() = OnboardingStep.entries.filter {
            showModeChoice || it != OnboardingStep.RECORDING_MODE
        }

    val canContinue: Boolean
        get() = when (step) {
            OnboardingStep.WELCOME -> true
            OnboardingStep.TRY_IT -> micGranted
            OnboardingStep.ACCESSIBILITY -> accessibilityReady
            OnboardingStep.BATTERY -> true
            OnboardingStep.RECORDING_MODE -> true
        }

    val showMicrophoneTapCue: Boolean
        get() = step == OnboardingStep.TRY_IT &&
            modelReady &&
            !recording &&
            transcript.isBlank()
}

@Composable
fun OnboardingScreen(
    state: OnboardingState,
    onBack: () -> Unit,
    onContinue: () -> Unit,
    onTry: () -> Unit,
    onAppSettings: () -> Unit,
    onAccessibility: () -> Unit,
    onBattery: () -> Unit,
    onRecordingMode: (String) -> Unit,
) {
    val index = state.steps.indexOf(state.step)
    ProductWizardShell(
        title = stringResource(R.string.app_name),
        step = index + 1,
        totalSteps = state.steps.size,
        canContinue = state.canContinue,
        continueLabel = stringResource(
            if (state.step == state.steps.last()) R.string.action_done else R.string.action_next,
        ),
        onBack = if (index > 0) onBack else null,
        onContinue = onContinue,
    ) {
        when (state.step) {
            OnboardingStep.WELCOME -> WelcomeStep()
            OnboardingStep.TRY_IT -> TryStep(state, onTry, onAppSettings)
            OnboardingStep.ACCESSIBILITY -> AccessibilityStep(state, onAccessibility)
            OnboardingStep.BATTERY -> BatteryStep(state, onBattery)
            OnboardingStep.RECORDING_MODE -> RecordingModeStep(state.recordingMode, onRecordingMode)
        }
    }
}

@Composable
private fun OnboardingPage(content: @Composable () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 24.dp, vertical = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        content = { content() },
    )
}

@Composable
private fun WelcomeStep() = OnboardingPage {
    Spacer(Modifier.height(16.dp))
    BirdHero()
    Spacer(Modifier.height(32.dp))
    OnboardingHeading(stringResource(R.string.onb_welcome_title))
    Spacer(Modifier.height(16.dp))
    OnboardingBody(
        value = stringResource(R.string.onb_welcome_body),
        textAlign = TextAlign.Start,
    )
    Spacer(Modifier.height(16.dp))
    OnboardingNote(
        value = stringResource(R.string.onb_welcome_note),
        textAlign = TextAlign.Start,
    )
}

@Composable
private fun TryStep(
    state: OnboardingState,
    onTry: () -> Unit,
    onAppSettings: () -> Unit,
) = OnboardingPage {
    Spacer(Modifier.height(16.dp))
    val bubbleColors = MaterialTheme.colorScheme
    Box(
        modifier = Modifier.fillMaxWidth().height(96.dp),
        contentAlignment = Alignment.Center,
    ) {
        AndroidView(
            factory = { context ->
                BubbleView(context).apply { configureForOnboarding() }
            },
            update = { bubble ->
                bubble.configureForOnboarding()
                bubble.setThemeColors(
                    BubbleThemeColors(
                        primary = bubbleColors.primary.toArgb(),
                        onPrimary = bubbleColors.onPrimary.toArgb(),
                        primaryContainer = bubbleColors.primaryContainer.toArgb(),
                        onPrimaryContainer = bubbleColors.onPrimaryContainer.toArgb(),
                        onSurface = bubbleColors.onSurface.toArgb(),
                        surfaceContainerHigh = bubbleColors.surfaceContainerHigh.toArgb(),
                        error = bubbleColors.error.toArgb(),
                    ),
                )
                bubble.setIdlePulseColor(bubbleColors.primary.toArgb())
                bubble.setRecording(state.recording)
                bubble.setIdlePulse(state.showMicrophoneTapCue)
                bubble.isEnabled = state.modelReady
                bubble.setOnClickListener { if (state.modelReady) onTry() }
            },
            modifier = Modifier.size(96.dp),
        )
    }
    Spacer(Modifier.height(24.dp))
    Text(
        text = when {
            !state.modelReady -> stringResource(R.string.onb_try_preparing)
            state.recording -> stringResource(R.string.onb_try_tap_to_stop)
            state.micGranted -> stringResource(R.string.onb_try_tap_to_start)
            else -> stringResource(R.string.onb_try_tap_to_grant)
        },
        modifier = Modifier.fillMaxWidth(),
        style = MaterialTheme.typography.titleMedium,
        textAlign = TextAlign.Center,
        color = MaterialTheme.colorScheme.onSurface,
    )
    if (!state.micGranted && !state.micDenied) {
        Spacer(Modifier.height(6.dp))
        OnboardingNote(
            value = stringResource(R.string.onb_try_grant_hint),
            textAlign = TextAlign.Center,
        )
    }
    Spacer(Modifier.height(24.dp))
    Surface(
        modifier = Modifier.fillMaxWidth().height(180.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
        shape = MaterialTheme.shapes.extraLarge,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
    ) {
        Text(
            text = state.transcript.ifBlank {
                stringResource(R.string.onb_try_result_placeholder)
            },
            modifier = Modifier.padding(20.dp),
            style = MaterialTheme.typography.bodyLarge,
            color = if (state.transcript.isBlank()) {
                MaterialTheme.colorScheme.onSurfaceVariant
            } else {
                MaterialTheme.colorScheme.onSurface
            },
        )
    }
    if (state.transcript.isNotBlank()) {
        Spacer(Modifier.height(16.dp))
        Text(
            stringResource(R.string.onb_try_done),
            modifier = Modifier.fillMaxWidth(),
            style = MaterialTheme.typography.titleSmall,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.primary,
        )
    }
    if (state.micDenied) {
        Spacer(Modifier.height(12.dp))
        Button(
            onClick = onAppSettings,
            shape = MaterialTheme.shapes.extraLarge,
        ) {
            Text(stringResource(R.string.onb_try_open_settings))
        }
    }
}

@Composable
private fun AccessibilityStep(
    state: OnboardingState,
    onAccessibility: () -> Unit,
) = OnboardingPage {
    Spacer(Modifier.height(16.dp))
    BirdHero()
    Spacer(Modifier.height(32.dp))
    OnboardingHeading(stringResource(R.string.onb_accessibility_title))
    Spacer(Modifier.height(16.dp))
    OnboardingBody(
        value = if (state.accessibilityReady) {
            stringResource(R.string.onb_accessibility_body_enabled)
        } else {
            stringResource(R.string.onb_accessibility_body_pending)
        },
        textAlign = TextAlign.Start,
    )
    if (!state.accessibilityReady) {
        Spacer(Modifier.height(20.dp))
        Column(horizontalAlignment = Alignment.Start) {
            OnboardingChecklistRow(
                icon = PhosphorIcons.Regular.CheckCircle,
                text = stringResource(R.string.onb_accessibility_do_row),
                color = MaterialTheme.colorScheme.primary,
            )
            Spacer(Modifier.height(10.dp))
            OnboardingChecklistRow(
                icon = PhosphorIcons.Regular.XCircle,
                text = stringResource(R.string.onb_accessibility_dont_row),
                color = MaterialTheme.colorScheme.error,
            )
        }
        Spacer(Modifier.height(24.dp))
        Button(
            onClick = onAccessibility,
            shape = MaterialTheme.shapes.extraLarge,
        ) {
            Text(stringResource(R.string.onb_accessibility_open))
        }
    }
}

@Composable
private fun BatteryStep(
    state: OnboardingState,
    onBattery: () -> Unit,
) = OnboardingPage {
    Spacer(Modifier.height(16.dp))
    BirdHero()
    Spacer(Modifier.height(32.dp))
    OnboardingHeading(stringResource(R.string.onb_battery_title))
    Spacer(Modifier.height(16.dp))
    OnboardingBody(
        value = if (state.batteryReady) {
            stringResource(R.string.onb_battery_body_done)
        } else {
            stringResource(R.string.onb_battery_body)
        },
        textAlign = TextAlign.Start,
    )
    if (!state.batteryReady) {
        Spacer(Modifier.height(24.dp))
        Button(
            onClick = onBattery,
            shape = MaterialTheme.shapes.extraLarge,
            colors = ButtonDefaults.filledTonalButtonColors(),
        ) {
            Text(stringResource(R.string.onb_battery_open))
        }
        Spacer(Modifier.height(24.dp))
        OnboardingNote(
            value = stringResource(R.string.onb_battery_skip_note),
            textAlign = TextAlign.Start,
        )
    }
}

@Composable
private fun BirdHero() {
    Icon(
        imageVector = ImageVector.vectorResource(R.drawable.logo_bird),
        contentDescription = null,
        modifier = Modifier.size(96.dp),
        tint = MaterialTheme.colorScheme.primary,
    )
}

@Composable
private fun OnboardingChecklistRow(
    icon: ImageVector,
    text: String,
    color: androidx.compose.ui.graphics.Color,
) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, contentDescription = null, modifier = Modifier.size(24.dp), tint = color)
        Spacer(Modifier.size(12.dp))
        Text(text, style = MaterialTheme.typography.bodyMedium, color = color)
    }
}

@Composable
private fun OnboardingHeading(value: String) {
    Text(
        value,
        style = MaterialTheme.typography.headlineSmall,
        textAlign = TextAlign.Center,
    )
}

@Composable
private fun OnboardingBody(
    value: String,
    textAlign: TextAlign,
) {
    Text(
        value,
        modifier = Modifier.fillMaxWidth(),
        style = MaterialTheme.typography.bodyLarge,
        textAlign = textAlign,
        color = MaterialTheme.colorScheme.onSurface,
    )
}

@Composable
private fun OnboardingNote(
    value: String,
    textAlign: TextAlign,
) {
    Text(
        value,
        modifier = Modifier.fillMaxWidth(),
        style = MaterialTheme.typography.bodySmall,
        textAlign = textAlign,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
