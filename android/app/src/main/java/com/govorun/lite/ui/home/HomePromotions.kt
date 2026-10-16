package com.govorun.lite.ui.home

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.MagicWand
import com.adamglin.phosphoricons.regular.Info
import com.adamglin.phosphoricons.regular.SlidersHorizontal
import com.adamglin.phosphoricons.regular.X
import com.govorun.lite.R
import com.govorun.lite.stylization.CustomStylePolicy
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.StyleBetaNotice
import com.govorun.lite.stylization.titleRes
import com.product.ui.ProductTokens
import com.product.ui.SettingsInfoRow

@Composable
internal fun StylePickerCard(
    offer: StylizationOfferState,
    onEvent: (HomeEvent) -> Unit,
    justEnabled: Boolean = false,
    onlineServicePaused: Boolean = false,
) {
    val servicePaused = onlineServicePaused && offer.enabled && !offer.selfHosted && offer.pauseMessage == null
    var expanded by rememberSaveable { mutableStateOf(justEnabled || !offer.enabled) }
    LaunchedEffect(justEnabled) { if (justEnabled) expanded = true }
    var editedPreset by remember { mutableStateOf<CustomStylePreset?>(null) }
    var editorVisible by rememberSaveable { mutableStateOf(false) }
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        ),
    ) {
        Column {
            FeedSectionHeader(
                title = stringResource(R.string.stylization_title),
                icon = PhosphorIcons.Regular.MagicWand,
                trailingText = if (offer.enabled) {
                    if (offer.selfHosted) "Свой сервер"
                    else stringResource(offer.selectedStyle.titleRes) + if (offer.emojiEnabled) " · с эмодзи" else ""
                } else {
                    stringResource(R.string.main_styles_disabled)
                },
                expanded = expanded,
                onToggle = { expanded = !expanded },
            )
            if (servicePaused) {
                Text("Улучшение текста временно недоступно. Обычная диктовка работает.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Lg, vertical = ProductTokens.Spacing.Sm))
            }
            if (!servicePaused && !offer.processingFailed) offer.processingMessage?.let { message ->
                Text(message, style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Lg, vertical = ProductTokens.Spacing.Sm))
            }
            offer.pauseMessage?.let { message ->
                Column(modifier = Modifier.padding(ProductTokens.Spacing.Lg)) {
                    Text(message, style = MaterialTheme.typography.bodyMedium)
                    Text("Диктовка работает без стилизации.", style = MaterialTheme.typography.bodySmall)
                    TextButton(onClick = { onEvent(HomeEvent.OpenSubscription) }) {
                        Text("Подписка и лимиты")
                    }
                }
            }
            AnimatedVisibility(expanded) {
                Column {
                    CompactDivider()
                    StyleControls(
                        offer = offer,
                        onEvent = onEvent,
                        onAdd = {
                            editedPreset = null
                            editorVisible = true
                        },
                        onEdit = {
                            editedPreset = it
                            editorVisible = true
                        },
                    )
                    StyleSettingsAction(offer.selfHosted) { onEvent(HomeEvent.OpenStylization) }
                }
            }
            if (!servicePaused && offer.processingFailed) offer.processingMessage?.let { message ->
                CompactDivider()
                Surface(color = MaterialTheme.colorScheme.surfaceContainer) {
                    SettingsInfoRow(
                        icon = PhosphorIcons.Regular.Info,
                        title = "Последняя диктовка — без улучшения",
                        summary = message,
                        trailing = {
                            IconButton(onClick = { onEvent(HomeEvent.DismissStyleFeedback(offer.processingRevision)) }) {
                                Icon(PhosphorIcons.Regular.X, contentDescription = "Скрыть сообщение",
                                    modifier = Modifier.size(ProductTokens.Size.TopBarActionIcon))
                            }
                        },
                    )
                }
            }
        }
    }
    if (editorVisible) {
        CustomStylePresetDialog(
            preset = editedPreset,
            unlimitedInstruction = offer.selfHosted,
            onDismiss = { editorVisible = false },
            onSave = { id, name, instruction ->
                onEvent(HomeEvent.SaveCustomStyle(id, name, instruction))
                editorVisible = false
            },
            onDelete = editedPreset?.let { preset ->
                {
                    onEvent(HomeEvent.DeleteCustomStyle(preset.id))
                    editorVisible = false
                }
            },
        )
    }
}

@Composable
private fun StyleSettingsAction(selfHosted: Boolean, onOpenSettings: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(
                start = ProductTokens.Spacing.Xs,
                end = ProductTokens.Spacing.Xs,
                bottom = ProductTokens.Spacing.Xs,
            ),
        horizontalArrangement = Arrangement.End,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = onOpenSettings, enabled = !selfHosted) {
            Icon(PhosphorIcons.Regular.SlidersHorizontal, contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.SlotIcon))
            Spacer(Modifier.width(ProductTokens.Spacing.Sm))
            Text("Настройки текста")
        }
    }
}

@Composable
private fun StyleControls(
    offer: StylizationOfferState,
    onEvent: (HomeEvent) -> Unit,
    onAdd: () -> Unit,
    onEdit: (CustomStylePreset) -> Unit,
) {
    if (offer.selfHosted) {
        Text("Стили Говоруна недоступны, пока выбран свой сервер.",
            modifier = Modifier.padding(horizontal = ProductTokens.Spacing.Section,
                vertical = ProductTokens.Spacing.Sm),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    FlowRow(
        modifier = Modifier
            .fillMaxWidth()
            .padding(
                start = ProductTokens.Spacing.Section,
                end = ProductTokens.Spacing.Section,
                top = ProductTokens.Spacing.Sm,
            ),
        horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Xxs),
    ) {
        TextStyle.entries.filterNot { it == TextStyle.CUSTOM }.forEach { style ->
            FilterChip(
                enabled = !offer.selfHosted,
                selected = !offer.selfHosted && offer.enabled && style == offer.selectedStyle,
                onClick = { onEvent(HomeEvent.SelectStyle(style)) },
                label = {
                    Text(
                        stringResource(style.titleRes),
                        style = MaterialTheme.typography.labelMedium,
                    )
                },
                shape = RoundedCornerShape(ProductTokens.Radius.Md),
            )
        }
        offer.customStyles.forEach { preset ->
            val selected = !offer.selfHosted && offer.enabled && offer.selectedStyle == TextStyle.CUSTOM &&
                offer.activeCustomStyleId == preset.id
            FilterChip(
                enabled = !offer.selfHosted,
                selected = selected,
                onClick = {
                    if (selected) onEdit(preset)
                    else onEvent(HomeEvent.SelectCustomStyle(preset.id))
                },
                label = { Text(preset.name, style = MaterialTheme.typography.labelMedium) },
                shape = RoundedCornerShape(ProductTokens.Radius.Md),
            )
        }
        if (offer.canCreateCustomStyle) {
            FilterChip(
                enabled = !offer.selfHosted,
                selected = false,
                onClick = onAdd,
                label = { Text("+") },
                shape = RoundedCornerShape(ProductTokens.Radius.Md),
            )
        }
        FilterChip(
            selected = !offer.enabled,
            onClick = { onEvent(HomeEvent.DisableStylization) },
            label = {
                Text(
                    stringResource(R.string.main_styles_disabled),
                )
            },
            shape = RoundedCornerShape(ProductTokens.Radius.Md),
        )
        if (offer.selfHosted) {
            FilterChip(
                enabled = offer.canEnableDirectly,
                selected = offer.enabled,
                onClick = { onEvent(HomeEvent.SelectSelfHostedGeneral) },
                label = { Text("Свой сервер") },
                shape = RoundedCornerShape(ProductTokens.Radius.Md),
            )
        }
    }

}

@Composable
private fun CustomStylePresetDialog(
    preset: CustomStylePreset?,
    unlimitedInstruction: Boolean,
    onDismiss: () -> Unit,
    onSave: (String?, String, String) -> Unit,
    onDelete: (() -> Unit)?,
) {
    var name by rememberSaveable(preset?.id) { mutableStateOf(preset?.name.orEmpty()) }
    var instruction by rememberSaveable(preset?.id) {
        mutableStateOf(preset?.instruction.orEmpty())
    }
    val normalized = if (unlimitedInstruction) instruction.trim()
        else CustomStylePolicy.normalize(instruction)
    val validInstruction = if (unlimitedInstruction) normalized.length >= 3
        else CustomStylePolicy.isValid(normalized)
    val valid = name.isNotBlank() && validInstruction
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(if (preset == null) "Новый стиль" else "Изменить стиль")
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Lg)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = CustomStylePolicy.input(it, "name") },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Название кнопки") },
                    singleLine = true,
                    supportingText = {
                        Text("${name.length}/${CustomStylePresets.MAX_NAME_CHARS}")
                    },
                )
                OutlinedTextField(
                    value = instruction,
                    onValueChange = {
                        instruction = if (unlimitedInstruction) it
                            else CustomStylePolicy.input(it)
                    },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Что сделать с текстом") },
                    placeholder = {
                        Text(stringResource(R.string.stylization_custom_rule_hint))
                    },
                    supportingText = {
                        Text(
                            if (unlimitedInstruction) "${instruction.length} символов"
                            else "${CustomStylePolicy.characters(instruction)}/${CustomStylePolicy.MAX_INSTRUCTION_CHARS}",
                        )
                    },
                    minLines = 3,
                    maxLines = 5,
                )
            }
        },
        confirmButton = {
            TextButton(
                enabled = valid,
                onClick = { onSave(preset?.id, name, normalized) },
            ) {
                Text("Сохранить")
            }
        },
        dismissButton = {
            Row {
                if (onDelete != null) {
                    TextButton(onClick = onDelete) { Text("Удалить") }
                }
                TextButton(onClick = onDismiss) { Text("Отмена") }
            }
        },
    )
}

@Composable
internal fun SubscriptionCard(
    offer: StylizationOfferState,
    onEvent: (HomeEvent) -> Unit,
) {
    Card(
        modifier = Modifier.feedCard(),
        shape = RoundedCornerShape(ProductTokens.Radius.Card),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer,
        ),
    ) {
        if (!offer.signedIn && !offer.selfHosted && !offer.termsRequired) {
            StyleIntroduction(offer, onEvent)
        } else {
            Column(
                modifier = Modifier.fillMaxWidth().padding(ProductTokens.Spacing.Section),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                ) {
                    Surface(
                        shape = CircleShape,
                        color = MaterialTheme.colorScheme.primary,
                    ) {
                        Icon(
                            PhosphorIcons.Regular.MagicWand,
                            contentDescription = null,
                            modifier = Modifier.padding(9.dp).size(20.dp),
                            tint = MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                    Column(
                        modifier = Modifier.weight(1f),
                    ) {
                        Text(
                            if (offer.termsRequired) "Обновлены условия" else "Улучшение текста",
                            style = MaterialTheme.typography.titleMedium,
                        )
                        if (!offer.selfHosted) StyleBetaNotice(compact = true)
                        Text(
                            offer.accessLabel,
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onPrimaryContainer,
                        )
                    }
                }
                Button(
                    enabled = offer.actionEnabled,
                    onClick = { onEvent(offer.accessEvent) },
                    modifier = Modifier.align(Alignment.End),
                ) {
                    Text(offer.actionTitle)
                }
            }
        }
    }
}

@Composable
private fun StyleIntroduction(offer: StylizationOfferState, onEvent: (HomeEvent) -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(ProductTokens.Spacing.Section),
        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            Icon(
                PhosphorIcons.Regular.MagicWand,
                contentDescription = null,
                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
            )
            Text("Улучшение текста", style = MaterialTheme.typography.titleMedium)
        }
        StyleBetaNotice(compact = true)
        Text(
            offer.accessLabel,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onPrimaryContainer,
        )
        Button(
            enabled = offer.actionEnabled,
            onClick = { onEvent(offer.accessEvent) },
            modifier = Modifier.align(Alignment.End),
        ) { Text(offer.actionTitle) }
    }
}
