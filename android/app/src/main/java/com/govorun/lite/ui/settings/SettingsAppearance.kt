package com.govorun.lite.ui.settings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import com.govorun.lite.overlay.BirdColors
import androidx.compose.material3.Icon
import androidx.compose.ui.platform.LocalContext
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Check
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
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
        BirdColorSetting(state.birdColor) {
            onEvent(SettingsEvent.Change(SettingsChange.SetBirdColor(it)))
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
            title = "Прозрачность в покое",
            valueText = "${(state.bubbleAlpha * 100).roundToInt()}%",
            value = state.bubbleAlpha,
            range = Prefs.BUBBLE_ALPHA_MIN..Prefs.BUBBLE_ALPHA_MAX,
            steps = 13,
            onValueChange = { onEvent(SettingsEvent.Change(SettingsChange.SetBubbleAlpha(it))) },
        )
        SliderSetting(
            title = "Прозрачность при записи",
            valueText = "${(state.recordingAlpha * 100).roundToInt()}%",
            value = state.recordingAlpha,
            range = Prefs.BUBBLE_ALPHA_MIN..Prefs.BUBBLE_ALPHA_MAX,
            steps = 13,
            onValueChange = {
                onEvent(SettingsEvent.Change(SettingsChange.SetBubbleRecordingAlpha(it)))
            },
        )
        SettingsDivider()
        SliderSetting(
            title = "Отступ от края",
            valueText = if (state.edgeMarginCustomized) "${state.edgeMargin} dp" else "Стандартный",
            value = state.edgeMargin.coerceAtMost(state.edgeMarginMax).toFloat(),
            range = Prefs.BUBBLE_EDGE_MARGIN_MIN.toFloat()..
                state.edgeMarginMax.toFloat(),
            steps = (state.edgeMarginMax - Prefs.BUBBLE_EDGE_MARGIN_MIN) /
                Prefs.BUBBLE_EDGE_MARGIN_STEP - 1,
            onValueChange = {
                onEvent(SettingsEvent.Change(SettingsChange.SetEdgeMargin(it.roundToInt())))
            },
        )
        SettingsDivider()
        SettingsLinkRow(
            title = "Сбросить внешний вид",
            summary = "Сторона, цвет, размер, прозрачность и отступ — как при установке",
            onClick = { onEvent(SettingsEvent.ResetAppearance) },
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
@OptIn(ExperimentalMaterial3Api::class)
private fun BubbleSideSetting(
    selected: String,
    onSelected: (String) -> Unit,
) {
    val options = listOf(
        Prefs.BUBBLE_SIDE_LEFT to "Слева",
        Prefs.BUBBLE_SIDE_RIGHT to "Справа",
    )
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = GovorunSpacing.xLarge, vertical = GovorunSpacing.medium),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("Сторона экрана", modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
        SingleChoiceSegmentedButtonRow {
            options.forEachIndexed { index, (value, label) ->
                SegmentedButton(
                    selected = value == selected,
                    onClick = { onSelected(value) },
                    shape = SegmentedButtonDefaults.itemShape(index, options.size),
                ) { Text(label) }
            }
        }
    }
}

/** The strong (recording) tone of a bird colour and the colour drawn on it. */
@Composable
private fun birdAccent(id: String): Pair<Color, Color> {
    val tone = BirdColors.tone(LocalContext.current, id)
    return Color(tone.fill) to Color(tone.onFill)
}

/** A Material colour swatch: a filled circle, ringed and ticked when selected. */
@Composable
private fun BirdSwatch(id: String, selected: Boolean, size: Dp) {
    val (accent, onAccent) = birdAccent(id)
    Box(
        modifier = Modifier
            .size(size)
            .then(
                if (selected) Modifier.border(2.dp, MaterialTheme.colorScheme.primary, CircleShape).padding(4.dp)
                else Modifier,
            )
            .clip(CircleShape)
            .background(accent),
        contentAlignment = Alignment.Center,
    ) {
        if (selected) {
            Icon(
                PhosphorIcons.Regular.Check,
                contentDescription = null,
                tint = onAccent,
                modifier = Modifier.size(size * 0.4f),
            )
        }
    }
}

@Composable
@OptIn(ExperimentalMaterial3Api::class)
private fun BirdColorSetting(
    selected: String,
    onSelected: (String) -> Unit,
) {
    var sheetOpen by remember { mutableStateOf(false) }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { sheetOpen = true }
            .padding(horizontal = GovorunSpacing.xLarge, vertical = GovorunSpacing.large),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text("Цвет птички", style = MaterialTheme.typography.bodyMedium)
            Text(
                BirdColors.label(selected),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        BirdSwatch(selected, selected = false, size = 28.dp)
    }
    if (sheetOpen) {
        ModalBottomSheet(
            onDismissRequest = { sheetOpen = false },
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        ) {
            Column(modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Screen).padding(bottom = ProductTokens.Spacing.Screen)) {
                Text("Цвет птички", style = MaterialTheme.typography.titleLarge)
                Spacer(Modifier.height(GovorunSpacing.large))
                BirdColors.ids.chunked(3).forEach { row ->
                    Row(modifier = Modifier.fillMaxWidth().selectableGroup()) {
                        row.forEach { id ->
                            Column(
                                modifier = Modifier
                                    .weight(1f)
                                    .clip(MaterialTheme.shapes.medium)
                                    .selectable(
                                        selected = id == selected,
                                        role = Role.RadioButton,
                                        onClick = {
                                            onSelected(id)
                                            sheetOpen = false
                                        },
                                    )
                                    .padding(vertical = GovorunSpacing.medium),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                BirdSwatch(id, selected = id == selected, size = 48.dp)
                                Text(
                                    BirdColors.label(id),
                                    style = MaterialTheme.typography.labelLarge,
                                    textAlign = TextAlign.Center,
                                    modifier = Modifier.padding(top = GovorunSpacing.small),
                                )
                            }
                        }
                        repeat(3 - row.size) { Spacer(Modifier.weight(1f)) }
                    }
                }
            }
        }
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
