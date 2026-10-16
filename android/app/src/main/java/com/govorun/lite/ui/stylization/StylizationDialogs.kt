package com.govorun.lite.ui.stylization

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Duotone
import com.adamglin.phosphoricons.duotone.FileText
import com.govorun.lite.R
import com.govorun.lite.stylization.CustomStylePolicy
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.descriptionRes
import com.govorun.lite.stylization.titleRes
import com.product.ui.ProductDivider
import com.product.ui.ProductTokens
import com.product.ui.SettingsLinkRow
import com.product.ui.SettingsSection

@Composable
internal fun OnlineAccessCard(
    state: StylizationUiState,
    onOpenAccess: () -> Unit,
    onAcceptTerms: () -> Unit,
    onReadTerms: () -> Unit,
) {
    SettingsSection("Улучшение текста") {
        if (!state.hasAccess) {
            Text(
                "Для стилизации нужен доступ.",
                modifier = Modifier.padding(ProductTokens.Spacing.Section),
                style = MaterialTheme.typography.bodyMedium,
            )
            ProductDivider()
            Button(
                onClick = onOpenAccess,
                modifier = Modifier.padding(ProductTokens.Spacing.Section),
            ) { Text("Получить доступ") }
        } else {
            Text(
                "Для улучшения отправляется только готовый распознанный текст. Аудио и данные аккаунта не отправляются.",
                modifier = Modifier.padding(ProductTokens.Spacing.Section),
                style = MaterialTheme.typography.bodyMedium,
            )
            ProductDivider()
            SettingsLinkRow(
                icon = PhosphorIcons.Duotone.FileText,
                title = "Условия и политика",
                summary = "Что передаётся и кто обрабатывает текст",
                onClick = onReadTerms,
            )
            ProductDivider()
            Button(
                onClick = onAcceptTerms,
                modifier = Modifier.padding(ProductTokens.Spacing.Section),
            ) { Text("Принять и включить") }
        }
    }
}

@Composable
internal fun CustomInstructionDialog(
    preset: CustomStylePreset?,
    unlimitedInstruction: Boolean,
    onDismiss: () -> Unit,
    onSave: (String?, String, String, StyleOptions) -> Unit,
    onDelete: (() -> Unit)?,
) {
    var name by rememberSaveable(preset?.id) { mutableStateOf(preset?.name.orEmpty()) }
    var value by rememberSaveable(preset?.id) { mutableStateOf(preset?.instruction.orEmpty()) }
    var emoji by rememberSaveable(preset?.id) { mutableStateOf(preset?.resolvedEmojiMode ?: EmojiMode.OFF) }
    var reduced by rememberSaveable(preset?.id) { mutableStateOf(preset?.punctuationMode == PunctuationMode.LESS) }
    var profanity by rememberSaveable(preset?.id) { mutableStateOf(preset?.profanityFilterEnabled ?: false) }
    val options = StyleOptions(if (reduced) PunctuationMode.LESS else PunctuationMode.NORMAL,
        emoji.enabled, profanity, emoji)
    val normalized = if (unlimitedInstruction) value.trim() else CustomStylePolicy.normalize(value)
    val validRule = if (unlimitedInstruction) normalized.length >= 3
        else CustomStylePolicy.isValid(normalized)
    val invalidRule = normalized.isNotEmpty() && !validRule
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.stylization_custom_editor_title)) },
        text = {
            Column(modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
                Text(
                    stringResource(R.string.stylization_custom_editor_help),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = CustomStylePolicy.input(it, "name") },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Название кнопки") },
                    singleLine = true,
                )
                OutlinedTextField(
                    value = value,
                    onValueChange = {
                        value = if (unlimitedInstruction) it
                            else CustomStylePolicy.input(it)
                    },
                    modifier = Modifier.fillMaxWidth(),
                    placeholder = { Text(stringResource(R.string.stylization_custom_rule_hint)) },
                    supportingText = {
                        Text(if (invalidRule) {
                            stringResource(R.string.stylization_custom_rule_invalid)
                        } else if (unlimitedInstruction) {
                            "${value.length} символов"
                        } else {
                            "${CustomStylePolicy.characters(value)}/${CustomStylePolicy.MAX_INSTRUCTION_CHARS}"
                        })
                    },
                    isError = invalidRule,
                    minLines = 2,
                    maxLines = 4,
                )
                ProductDivider()
                CustomStyleOptionsEditor(options) {
                    emoji = it.resolvedEmojiMode
                    reduced = it.punctuation == PunctuationMode.LESS
                    profanity = it.profanityFilterEnabled
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSave(preset?.id, name, normalized, options) },
                enabled = name.isNotBlank() && validRule,
            ) { Text(stringResource(R.string.stylization_custom_rule_save)) }
        },
        dismissButton = {
            Row {
                if (onDelete != null) TextButton(onClick = onDelete) { Text("Удалить") }
                TextButton(onClick = onDismiss) {
                    Text(stringResource(R.string.stylization_custom_rule_cancel))
                }
            }
        },
    )
}

@Composable
internal fun StylePicker(
    selected: TextStyle,
    enabled: Boolean,
    selfHosted: Boolean,
    selfHostedStyleTitle: String,
    customStyles: List<CustomStylePreset>,
    activeCustomStyleId: String?,
    onDismiss: () -> Unit,
    onDisable: () -> Unit,
    onSelected: (TextStyle) -> Unit,
    onCustomStyle: (String) -> Unit,
    onSelfHostedGeneral: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.stylization_style_title)) },
        text = {
            Column {
                PickerRow(
                    selected = !enabled,
                    title = "Без стилизации",
                    description = "Вставлять обычный распознанный текст",
                    onClick = onDisable,
                )
                if (selfHosted) {
                    PickerRow(
                        selected = enabled && selfHostedStyleTitle == "Общий",
                        title = "Общий",
                        description = "Одна инструкция для всех случаев",
                        onClick = onSelfHostedGeneral,
                    )
                } else {
                    TextStyle.entries.filterNot { it == TextStyle.CUSTOM }.forEach { style ->
                        PickerRow(
                            selected = enabled && style == selected,
                            title = stringResource(style.titleRes),
                            description = stringResource(style.descriptionRes),
                            onClick = { onSelected(style) },
                        )
                    }
                }
                customStyles.filter { !selfHosted }.forEach { preset ->
                    PickerRow(
                        selected = enabled && selected == TextStyle.CUSTOM &&
                            activeCustomStyleId == preset.id,
                        title = preset.name,
                        description = preset.instruction,
                        onClick = { onDismiss(); onCustomStyle(preset.id) },
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.stylization_picker_close))
            }
        },
    )
}

@Composable
private fun PickerRow(
    selected: Boolean,
    title: String,
    description: String,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(vertical = ProductTokens.Spacing.Md),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg),
    ) {
        RadioButton(selected = selected, onClick = onClick)
        Column {
            Text(title, style = MaterialTheme.typography.bodyMedium)
            Text(
                description,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
