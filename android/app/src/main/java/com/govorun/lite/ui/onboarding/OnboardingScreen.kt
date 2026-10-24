package com.govorun.lite.ui.onboarding

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.CheckCircle
import com.adamglin.phosphoricons.regular.Clock
import com.adamglin.phosphoricons.regular.Microphone
import com.adamglin.phosphoricons.regular.PersonArmsSpread
import com.adamglin.phosphoricons.regular.XCircle
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.RecordingModePrefs
import com.product.ui.FeedCard
import com.product.ui.ProductDivider
import com.product.ui.ProductTokens
import com.product.ui.ProductWizardShell
import com.product.ui.SettingsLinkRow

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

/** One step as Echo lays it out: a hero on top, then cards and notes with even gaps. */
@Composable
internal fun OnboardingPage(content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        Spacer(Modifier.height(12.dp))
        content()
    }
}

/** A large title and a quiet centred line, as at the top of every Echo step, under an optional picture. */
@Composable
internal fun OnboardingHero(
    title: String,
    body: String?,
    visual: (@Composable () -> Unit)? = null,
) {
    Column(
        Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        visual?.let {
            it()
            Spacer(Modifier.height(4.dp))
        }
        Text(title, style = MaterialTheme.typography.headlineMedium, textAlign = TextAlign.Center)
        body?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}

/** The app icon itself on the first step, as Echo shows its own: the white release tile with the bird, in debug too. */
@Composable
private fun GovorunArt() {
    val shape = RoundedCornerShape(24.dp)
    Box(
        Modifier
            .size(104.dp)
            .clip(shape)
            .background(Color.White)
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, shape),
        contentAlignment = Alignment.Center,
    ) {
        // The adaptive foreground is drawn for a 108 dp canvas with a 72 dp safe zone, so it is scaled by 1.5.
        Image(painterResource(R.drawable.ic_launcher_foreground), contentDescription = null, modifier = Modifier.requiredSize(156.dp))
    }
}

@Composable
private fun WelcomeStep() = OnboardingPage {
    OnboardingHero(stringResource(R.string.onb_welcome_title), stringResource(R.string.onb_welcome_body)) { GovorunArt() }
    OnboardingNote(stringResource(R.string.onb_welcome_note), TextAlign.Center)
}

@Composable
private fun TryStep(
    state: OnboardingState,
    onTry: () -> Unit,
    onAppSettings: () -> Unit,
) = OnboardingPage {
    val status = when {
        !state.modelReady -> stringResource(R.string.onb_try_preparing)
        state.recording -> stringResource(R.string.onb_try_tap_to_stop)
        state.micGranted -> stringResource(R.string.onb_try_tap_to_start)
        else -> stringResource(R.string.onb_try_tap_to_grant)
    }
    OnboardingHero(stringResource(R.string.onb_try_title), status) { TryBubble(state, onTry) }
    if (!state.micGranted && !state.micDenied) {
        OnboardingNote(stringResource(R.string.onb_try_grant_hint), TextAlign.Center)
    }
    FeedCard {
        Text(
            text = state.transcript.ifBlank { stringResource(R.string.onb_try_result_placeholder) },
            modifier = Modifier.fillMaxWidth().heightIn(min = 140.dp).padding(ProductTokens.Spacing.Section),
            style = MaterialTheme.typography.bodyLarge,
            color = if (state.transcript.isBlank()) {
                MaterialTheme.colorScheme.onSurfaceVariant
            } else {
                MaterialTheme.colorScheme.onSurface
            },
        )
    }
    if (state.transcript.isNotBlank()) {
        Text(
            stringResource(R.string.onb_try_done),
            modifier = Modifier.fillMaxWidth(),
            style = MaterialTheme.typography.titleSmall,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.primary,
        )
    }
    if (state.micDenied) {
        FeedCard {
            SettingsLinkRow(PhosphorIcons.Regular.Microphone, "Разрешите микрофон", "Аудио остаётся на телефоне", onClick = onAppSettings)
        }
    }
}

@Composable
private fun TryBubble(state: OnboardingState, onTry: () -> Unit) {
    val bubbleColors = MaterialTheme.colorScheme
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
            bubble.setRecording(state.recording)
            bubble.setIdlePulse(state.showMicrophoneTapCue)
            bubble.isEnabled = state.modelReady
            bubble.setOnClickListener { if (state.modelReady) onTry() }
        },
        modifier = Modifier.size(96.dp),
    )
}

// Every step is a title over one card of the same rows as Settings and the home «Нужно настроить» card: the action
// is a row with a chevron, and a done step turns it into a green row, as the home «Всё готово» card.
@Composable
private fun AccessibilityStep(
    state: OnboardingState,
    onAccessibility: () -> Unit,
) = OnboardingPage {
    OnboardingHero(stringResource(R.string.onb_accessibility_title), stringResource(R.string.onb_accessibility_body_pending))
    FeedCard {
        Column {
            if (state.accessibilityReady) {
                OnboardingChecklistRow(PhosphorIcons.Regular.CheckCircle, "Говорун включён", successColor())
            } else {
                SettingsLinkRow(
                    PhosphorIcons.Regular.PersonArmsSpread,
                    "Включите Говоруна",
                    "Чтобы вставлять текст в активное поле",
                    onClick = onAccessibility,
                )
                ProductDivider()
                // Which service to switch on and which to leave off: the colours guard against picking the wrong one.
                OnboardingChecklistRow(PhosphorIcons.Regular.CheckCircle, stringResource(R.string.onb_accessibility_do_row), successColor())
                OnboardingChecklistRow(PhosphorIcons.Regular.XCircle, stringResource(R.string.onb_accessibility_dont_row), MaterialTheme.colorScheme.error)
            }
        }
    }
}

@Composable
private fun BatteryStep(
    state: OnboardingState,
    onBattery: () -> Unit,
) = OnboardingPage {
    OnboardingHero(stringResource(R.string.onb_battery_title), stringResource(R.string.onb_battery_body))
    FeedCard {
        if (state.batteryReady) {
            OnboardingChecklistRow(PhosphorIcons.Regular.CheckCircle, "Работа в фоне разрешена", successColor())
        } else {
            SettingsLinkRow(PhosphorIcons.Regular.Clock, "Разрешите работу в фоне", "Для стабильной диктовки", onClick = onBattery)
        }
    }
    if (!state.batteryReady) {
        OnboardingNote(stringResource(R.string.onb_battery_skip_note), TextAlign.Start)
    }
}

/** Green as on the home «Всё готово» card: the monochrome primary would make it plain white or black. */
@Composable
private fun successColor(): Color =
    if (MaterialTheme.colorScheme.background.luminance() > 0.5f) ProductTokens.Color.Success else ProductTokens.Color.SuccessDark

/** A settings-style row whose icon and text carry the meaning in colour. */
@Composable
private fun OnboardingChecklistRow(
    icon: ImageVector,
    text: String,
    color: Color,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = ProductTokens.Size.Row)
            .padding(horizontal = ProductTokens.Spacing.Section, vertical = ProductTokens.Spacing.Md),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, contentDescription = null, modifier = Modifier.size(20.dp), tint = color)
        Spacer(Modifier.width(ProductTokens.Spacing.Lg))
        Text(text, style = MaterialTheme.typography.bodyMedium, color = color)
    }
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
