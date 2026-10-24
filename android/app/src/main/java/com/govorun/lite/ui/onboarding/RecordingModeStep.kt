package com.govorun.lite.ui.onboarding

import android.view.View
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.RecordingModePrefs
import com.product.ui.FeedCard
import com.product.ui.ProductTokens

@Composable
internal fun RecordingModeStep(selectedMode: String, onSelect: (String) -> Unit) = OnboardingPage {
    OnboardingHero(
        "Выберите режим",
        "Если не знаете, что выбрать, оставьте режим по умолчанию. Позже его можно изменить в настройках.",
    )
    FeedCard {
        Column(Modifier.padding(ProductTokens.Spacing.Section).selectableGroup()) {
            ModeOption(
                title = "Фразой целиком", wholePhrase = true,
                description = "Вставка по нажатию на птичку.\nЭтот режим используется для улучшения текста.",
                selected = selectedMode == RecordingModePrefs.REPEAT_TAP,
                onSelect = { onSelect(RecordingModePrefs.REPEAT_TAP) },
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 20.dp),
                color = MaterialTheme.colorScheme.outlineVariant)
            ModeOption(
                title = "После каждой паузы", wholePhrase = false,
                description = "Фраза появляется в приложении после паузы.",
                selected = selectedMode == RecordingModePrefs.AFTER_PAUSE,
                onSelect = { onSelect(RecordingModePrefs.AFTER_PAUSE) },
            )
        }
    }
}

@Composable
private fun ModeOption(
    title: String,
    wholePhrase: Boolean,
    description: String,
    selected: Boolean,
    onSelect: () -> Unit,
) {
    Column(Modifier.fillMaxWidth().selectable(selected = selected,
        role = Role.RadioButton, onClick = onSelect)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            RadioButton(selected = selected, onClick = null, modifier = Modifier.size(48.dp))
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleMedium)
            }
            ModeBirdPreview(wholePhrase)
        }
        Text(description, modifier = Modifier.padding(start = 48.dp, top = 8.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun ModeBirdPreview(wholePhrase: Boolean) {
    val colors = MaterialTheme.colorScheme
    Box(Modifier.width(114.dp).height(60.dp), contentAlignment = Alignment.Center) {
        AndroidView(
            factory = { context ->
                BubbleView(context).apply {
                    configureForOnboarding(compactModePreview = true)
                    importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    isClickable = false
                    isFocusable = false
                }
            },
            update = { bubble ->
                bubble.setThemeColors(BubbleThemeColors(
                    primary = colors.primary.toArgb(), onPrimary = colors.onPrimary.toArgb(),
                    primaryContainer = colors.primaryContainer.toArgb(),
                    onPrimaryContainer = colors.onPrimaryContainer.toArgb(),
                    onSurface = colors.onSurface.toArgb(),
                    surfaceContainerHigh = colors.surfaceContainerHigh.toArgb(),
                    error = colors.error.toArgb(),
                ))
                bubble.setPresentation(if (wholePhrase) BubbleView.Presentation.CONFIRM_RECORDING
                    else BubbleView.Presentation.LEGACY_RECORDING, anchorOnLeft = false)
            },
        )
    }
}
