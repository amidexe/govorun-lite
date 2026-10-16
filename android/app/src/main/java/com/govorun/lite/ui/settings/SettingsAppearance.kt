package com.govorun.lite.ui.settings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import com.govorun.lite.model.settings.SettingsChange
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.govorun.lite.overlay.BubbleView
import com.govorun.lite.overlay.BubbleThemeColors
import com.govorun.lite.ui.design.GovorunSpacing
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import com.product.ui.ProductTokens
import kotlin.math.roundToInt

@Composable
internal fun AppearanceSettings(
    state: SettingsUiState,
    onEvent: (SettingsEvent) -> Unit,
) {
    SettingsSection("Внешний вид") {
        BubblePreview(state)
        SettingsDivider()
        BubbleSideSetting(state.bubbleSide) {
            onEvent(SettingsEvent.Change(SettingsChange.SetBubbleSide(it)))
        }
        SettingsDivider()
        SliderSetting(
            title = "Размер",
            valueText = "${(state.bubbleSize * 100).roundToInt()}%",
            value = state.bubbleSize,
            range = Prefs.BUBBLE_SIZE_MIN..Prefs.BUBBLE_SIZE_MAX,
            steps = 13,
            onValueChange = { onEvent(SettingsEvent.Change(SettingsChange.SetBubbleSize(it))) },
        )
        SettingsDivider()
        SliderSetting(
            title = "Прозрачность фона",
            valueText = "${(state.bubbleAlpha * 100).roundToInt()}%",
            value = state.bubbleAlpha,
            range = Prefs.BUBBLE_ALPHA_MIN..Prefs.BUBBLE_ALPHA_MAX,
            steps = 13,
            onValueChange = { onEvent(SettingsEvent.Change(SettingsChange.SetBubbleAlpha(it))) },
        )
        SettingsDivider()
        SliderSetting(
            title = "Отступ от края",
            valueText = if (state.edgeMarginCustomized) "${state.edgeMargin} dp" else "Стандартный",
            value = state.edgeMargin.toFloat(),
            range = Prefs.BUBBLE_EDGE_MARGIN_MIN.toFloat()..
                Prefs.BUBBLE_EDGE_MARGIN_MAX.toFloat(),
            steps = 8,
            onValueChange = {
                onEvent(SettingsEvent.Change(SettingsChange.SetEdgeMargin(it.roundToInt())))
            },
        )
    }
    SettingsSection("Управление") {
        RecordingModeSetting(state.recordingMode) {
            onEvent(SettingsEvent.Change(SettingsChange.SetRecordingMode(it)))
        }
        SettingsDivider()
        SettingsSwitchRow(
            title = "Вибрация при нажатии",
            summary = "Отклик при записи и двойной сигнал при ошибке",
            checked = state.haptics,
            onCheckedChange = { onEvent(SettingsEvent.Change(SettingsChange.SetHaptics(it))) },
        )
        SettingsDivider()
        SettingsSwitchRow(
            title = "Зафиксировать позицию",
            summary = "Защищает птичку от случайного перемещения",
            checked = state.positionLocked,
            onCheckedChange = { onEvent(SettingsEvent.Change(SettingsChange.SetPositionLocked(it))) },
        )
    }
    SettingsSection("Где показывать птичку") {
        SettingsSwitchRow(
            title = "Скрывать в строках поиска",
            summary = "Не мешать, когда вы ищете что-либо в приложении",
            checked = state.hideInSearch,
            onCheckedChange = { onEvent(SettingsEvent.Change(SettingsChange.SetHideInSearch(it))) },
        )
        SettingsDivider()
        SettingsLinkRow(
            title = "Выбрать приложения",
            summary = state.appFilterSummary,
            onClick = { onEvent(SettingsEvent.OpenAppFilter) },
        )
    }
}

@Composable
private fun RecordingModeSetting(
    selected: String,
    onSelected: (String) -> Unit,
) {
    var sheetOpen by remember { mutableStateOf(false) }
    val options = listOf(
        RecordingModePrefs.AFTER_PAUSE to "После паузы",
        RecordingModePrefs.REPEAT_TAP to "Повторным нажатием",
    )
    val current = options.firstOrNull { it.first == selected } ?: options.first()
    val summary = if (current.first == RecordingModePrefs.AFTER_PAUSE) {
        "Классический режим: после паузы, без стилизации"
    } else {
        "Нажмите птичку, договорите и снова нажмите её для отправки"
    }
    SettingsLinkRow(
        title = "Завершение диктовки",
        summary = summary,
        value = current.second,
        onClick = { sheetOpen = true },
    )
    if (sheetOpen) {
        SettingsChoiceSheet(
            title = "Завершение диктовки",
            selected = current,
            options = options,
            optionLabel = Pair<String, String>::second,
            onSelected = { onSelected(it.first) },
            onDismissRequest = { sheetOpen = false },
        )
    }
}

@Composable
private fun BubblePreview(state: SettingsUiState) {
    val scheme = MaterialTheme.colorScheme
    val bubbleColors = BubbleThemeColors(
        primary = scheme.primary.toArgb(),
        onPrimary = scheme.onPrimary.toArgb(),
        primaryContainer = scheme.primaryContainer.toArgb(),
        onPrimaryContainer = scheme.onPrimaryContainer.toArgb(),
        onSurface = scheme.onSurface.toArgb(),
        surfaceContainerHigh = scheme.surfaceContainerHigh.toArgb(),
        error = scheme.error.toArgb(),
    )
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 92.dp)
            .padding(horizontal = ProductTokens.Spacing.Section),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Предпросмотр", style = MaterialTheme.typography.bodyMedium)
            Text(
                "Размер и прозрачность",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Box(
            modifier = Modifier
                .width(132.dp)
                .height(92.dp),
            contentAlignment = Alignment.Center,
        ) {
            AndroidView(
                factory = { context ->
                    BubbleView(context).apply {
                        importantForAccessibility = android.view.View.IMPORTANT_FOR_ACCESSIBILITY_NO
                        setThemeColors(bubbleColors)
                        setIdleAlpha(state.bubbleAlpha)
                        setAnchoredLeft(false)
                    }
                },
                update = { bubble ->
                    bubble.setThemeColors(bubbleColors)
                    bubble.applySizeScaleFromPrefs()
                    bubble.setIdleAlpha(state.bubbleAlpha)
                    bubble.setAnchoredLeft(false)
                },
            )
        }
    }
}

@Composable
private fun BubbleSideSetting(
    selected: String,
    onSelected: (String) -> Unit,
) {
    var sheetOpen by remember { mutableStateOf(false) }
    val options = listOf(
        Prefs.BUBBLE_SIDE_LEFT to "Слева",
        Prefs.BUBBLE_SIDE_RIGHT to "Справа",
    )
    val current = options.firstOrNull { it.first == selected } ?: options.last()
    SettingsLinkRow(
        title = "Сторона экрана",
        value = current.second,
        onClick = { sheetOpen = true },
    )
    if (sheetOpen) {
        SettingsChoiceSheet(
            title = "Сторона экрана",
            selected = current,
            options = options,
            optionLabel = Pair<String, String>::second,
            onSelected = { onSelected(it.first) },
            onDismissRequest = { sheetOpen = false },
        )
    }
}

@Composable
@OptIn(ExperimentalMaterial3Api::class)
internal fun SliderSetting(
    title: String,
    valueText: String,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    steps: Int,
    onValueChange: (Float) -> Unit,
) {
    Column(
        modifier = Modifier.padding(
            start = GovorunSpacing.xLarge,
            end = GovorunSpacing.xLarge,
            top = GovorunSpacing.large,
            bottom = GovorunSpacing.small,
        ),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(title, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Spacer(Modifier.width(GovorunSpacing.medium))
            Text(
                valueText,
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Slider(
            value = value.coerceIn(range.start, range.endInclusive),
            onValueChange = onValueChange,
            valueRange = range,
            steps = steps,
            thumb = {
                Box(
                    Modifier
                        .size(18.dp)
                        .background(MaterialTheme.colorScheme.primary, CircleShape),
                )
            },
            track = {
                val span = range.endInclusive - range.start
                val progress = if (span == 0f) 0f else {
                    ((value - range.start) / span).coerceIn(0f, 1f)
                }
                val activeColor = MaterialTheme.colorScheme.primary
                val inactiveColor = MaterialTheme.colorScheme.surfaceContainerHighest
                Canvas(
                    Modifier
                        .fillMaxWidth()
                        .height(20.dp),
                ) {
                    val centerY = size.height / 2f
                    val stroke = 4.dp.toPx()
                    drawLine(
                        color = inactiveColor,
                        start = Offset(0f, centerY),
                        end = Offset(size.width, centerY),
                        strokeWidth = stroke,
                        cap = StrokeCap.Round,
                    )
                    drawLine(
                        color = activeColor,
                        start = Offset(0f, centerY),
                        end = Offset(size.width * progress, centerY),
                        strokeWidth = stroke,
                        cap = StrokeCap.Round,
                    )
                }
            },
        )
    }
}
