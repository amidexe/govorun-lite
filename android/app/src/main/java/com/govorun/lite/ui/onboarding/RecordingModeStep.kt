package com.govorun.lite.ui.onboarding

import android.view.View
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.R
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.util.RecordingModePrefs

@Composable
internal fun RecordingModeStep(selectedMode: String, onSelect: (String) -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())
            .padding(24.dp).selectableGroup(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Spacer(Modifier.height(8.dp))
        Icon(painterResource(R.drawable.logo_bird), contentDescription = null,
            modifier = Modifier.size(72.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(16.dp))
        Text("Выберите режим", style = MaterialTheme.typography.headlineSmall,
            textAlign = TextAlign.Center)
        Spacer(Modifier.height(16.dp))
        Text("Если не знаете, что выбрать, оставьте режим по умолчанию. Позже его можно изменить в настройках.",
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.height(24.dp))
        ModeOption(
            title = "Фразой целиком", wholePhrase = true,
            description = "Вставка по нажатию на птичку.\nЭтот режим используется для улучшения текста.",
            selected = selectedMode == RecordingModePrefs.REPEAT_TAP,
            onSelect = { onSelect(RecordingModePrefs.REPEAT_TAP) },
        )
        HorizontalDivider(modifier = Modifier.padding(vertical = 24.dp),
            color = MaterialTheme.colorScheme.outlineVariant)
        ModeOption(
            title = "После каждой паузы", wholePhrase = false,
            description = "Фраза появляется в приложении после паузы.",
            selected = selectedMode == RecordingModePrefs.AFTER_PAUSE,
            onSelect = { onSelect(RecordingModePrefs.AFTER_PAUSE) },
        )
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
                if (wholePhrase) Text("Рекомендуем", style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
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
