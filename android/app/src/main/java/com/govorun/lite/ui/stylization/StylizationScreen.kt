package com.govorun.lite.ui.stylization

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import com.govorun.lite.R
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.InlineStylizationConsentCard
import com.govorun.lite.stylization.descriptionRes
import com.govorun.lite.stylization.hasInlineStylizationConsent
import com.govorun.lite.stylization.titleRes
import com.product.ui.ProductChoiceSheet
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.SettingsLinkRow
import com.product.ui.SettingsSection
import com.product.ui.SettingsSwitchRow
import com.product.ui.productScreenPadding

internal data class StylizationUiState(
    val configured: Boolean = false,
    val enabled: Boolean = false,
    val canEnable: Boolean = false,
    val selfHosted: Boolean = false,
    val selfHostedStyleTitle: String = "Общий",
    val serverVerified: Boolean = false,
    val punctuationMode: PunctuationMode = PunctuationMode.NORMAL,
    val emojiMode: EmojiMode = EmojiMode.OFF,
    val profanityFilterEnabled: Boolean = false,
    val selectedStyle: TextStyle = TextStyle.FORMAL,
    val customInstruction: String = "",
    val customStyles: List<CustomStylePreset> = emptyList(),
    val canCreateCustomStyle: Boolean = false,
    val activeCustomStyleId: String? = null,
    val status: String = "",
    val notice: String? = null,
    val hasAccess: Boolean = false,
    val termsAccepted: Boolean = false,
    val processingReady: Boolean = false,
    val processingMessage: String? = null,
)

@Composable
internal fun StylizationScreen(
    state: StylizationUiState,
    openCustomEditorInitially: Boolean = false,
    onBack: () -> Unit,
    onToggle: (Boolean) -> Unit,
    onEmojiModeChange: (EmojiMode) -> Unit,
    onReducedPunctuationChange: (Boolean) -> Unit,
    onProfanityFilterChange: (Boolean) -> Unit,
    onStyle: (TextStyle) -> Unit,
    onSelfHostedGeneral: () -> Unit,
    onCustomStyle: (String) -> Unit,
    onSaveCustomStyle: (String?, String, String, com.govorun.lite.model.stylization.StyleOptions) -> Unit,
    onDeleteCustomStyle: (String) -> Unit,
    onCustomEditorVisibilityChange: (Boolean) -> Unit,
    onReadTerms: () -> Unit,
    onOpenAccess: () -> Unit,
    onAcceptTerms: () -> Unit,
    onAppRules: () -> Unit,
    onOpenSelfHostedServer: () -> Unit,
    onDismissNotice: () -> Unit,
) {
    var stylePickerVisible by rememberSaveable { mutableStateOf(false) }
    var emojiPickerVisible by rememberSaveable { mutableStateOf(false) }
    var editedCustomStyleId by rememberSaveable { mutableStateOf<String?>(null) }
    var customEditorVisible by rememberSaveable {
        mutableStateOf(openCustomEditorInitially)
    }
    LaunchedEffect(customEditorVisible) {
        onCustomEditorVisibilityChange(customEditorVisible)
    }
    val selectedStyleTitle = if (state.selectedStyle == TextStyle.CUSTOM) {
        state.customStyles.firstOrNull { it.id == state.activeCustomStyleId }?.name ?: "Свой стиль"
    } else stringResource(state.selectedStyle.titleRes)
    val consentVisible = hasInlineStylizationConsent()
    ProductScaffold(
        title = stringResource(R.string.stylization_title),
        onBack = onBack,
    ) { innerPadding ->
        if (consentVisible) {
            androidx.compose.foundation.lazy.LazyColumn(
                modifier = Modifier.fillMaxSize().padding(innerPadding), contentPadding = productScreenPadding(),
            ) { item { InlineStylizationConsentCard() } }
        } else LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            if (!state.hasAccess || !state.termsAccepted) {
                item {
                    OnlineAccessCard(
                        state = state,
                        onOpenAccess = onOpenAccess,
                        onAcceptTerms = onAcceptTerms,
                        onReadTerms = onReadTerms,
                    )
                }
            }
            item {
                SettingsSection("Стиль текста") {
                    if (state.selfHosted) {
                        Text("Эти настройки не применяются, пока включён свой сервер.",
                            modifier = Modifier.padding(ProductTokens.Spacing.Section),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        ProductDivider()
                    }
                    if (showProcessingUnavailable(state)) {
                        Text(
                            state.processingMessage ?: stringResource(R.string.stylization_service_temporarily_unavailable),
                            modifier = Modifier.padding(ProductTokens.Spacing.Section),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        ProductDivider()
                    }
                    SettingsLinkRow(
                        title = stringResource(R.string.stylization_style_title),
                        enabled = !state.selfHosted,
                        summary = if (!state.enabled) {
                            "Обычный распознанный текст без интернет-обработки"
                        } else {
                            stringResource(state.selectedStyle.descriptionRes)
                        },
                        value = if (!state.enabled) {
                            "Без стилизации"
                        } else {
                            selectedStyleTitle
                        },
                        onClick = {
                            when {
                                !state.selfHosted && state.hasAccess && state.termsAccepted ->
                                    stylePickerVisible = true
                                !state.hasAccess -> onOpenAccess()
                                else -> onAcceptTerms()
                            }
                        },
                    )
                }
            }
                item {
                    SettingsSection("Параметры стиля «$selectedStyleTitle»") {
                        SettingsLinkRow(
                            title = stringResource(R.string.stylization_emoji_title),
                            enabled = state.enabled && !state.selfHosted,
                            summary = emojiModeSummary(state.emojiMode),
                            value = emojiModeTitle(state.emojiMode),
                            onClick = { if (state.enabled) emojiPickerVisible = true },
                        )
                        ProductDivider()
                        SettingsSwitchRow(
                            title = stringResource(
                                R.string.stylization_reduced_punctuation_title,
                            ),
                            summary = stringResource(
                                R.string.stylization_reduced_punctuation_body,
                            ),
                            checked = state.punctuationMode == PunctuationMode.LESS,
                            enabled = state.enabled && !state.selfHosted,
                            onCheckedChange = onReducedPunctuationChange,
                        )
                        ProductDivider()
                        SettingsSwitchRow(
                            title = stringResource(R.string.stylization_profanity_filter_title),
                            summary = stringResource(R.string.stylization_profanity_filter_body),
                            checked = state.profanityFilterEnabled,
                            enabled = state.enabled && !state.selfHosted,
                            onCheckedChange = onProfanityFilterChange,
                        )
                    }
                }
                item {
                    SettingsSection("Правила приложений") {
                        SettingsLinkRow(
                            title = "Настроить правила",
                            enabled = !state.selfHosted,
                            summary = if (state.enabled) {
                                "Выбрать стиль для отдельных приложений"
                            } else {
                                "Сначала выберите стиль текста"
                            },
                            onClick = {
                                when {
                                    state.enabled -> onAppRules()
                                    state.hasAccess && state.termsAccepted ->
                                        stylePickerVisible = true
                                    !state.hasAccess -> onOpenAccess()
                                    else -> onAcceptTerms()
                                }
                            },
                        )
                    }
                }
            item {
                SettingsSection("Свои стили") {
                    state.customStyles.forEachIndexed { index, preset ->
                        if (index > 0) ProductDivider()
                        SettingsLinkRow(
                            title = preset.name,
                            enabled = !state.selfHosted,
                            summary = preset.instruction,
                            onClick = {
                                editedCustomStyleId = preset.id
                                customEditorVisible = true
                            },
                        )
                    }
                    if (state.canCreateCustomStyle) {
                        if (state.customStyles.isNotEmpty()) ProductDivider()
                        SettingsLinkRow(
                            title = "Добавить стиль",
                            enabled = !state.selfHosted,
                            summary = "До ${CustomStylePresets.MAX_PRESETS} собственных стилей",
                            onClick = {
                                editedCustomStyleId = null
                                customEditorVisible = true
                            },
                        )
                    }
                }
            }
            item {
                SettingsSection(stringResource(R.string.stylization_privacy_title)) {
                    Column(
                        modifier = Modifier.padding(ProductTokens.Spacing.Section),
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                    ) {
                            Text(
                                stringResource(R.string.stylization_privacy_legal),
                                style = MaterialTheme.typography.bodyMedium,
                            )
                        Text(
                            stringResource(R.string.stylization_privacy_body),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                        ProductDivider()
                        SettingsLinkRow(
                            title = stringResource(R.string.stylization_terms_title),
                            summary = stringResource(R.string.stylization_terms_summary),
                            onClick = onReadTerms,
                        )
                }
            }
        }
    }
    if (stylePickerVisible && !state.selfHosted) {
        StylePicker(
            selected = state.selectedStyle,
            enabled = state.enabled,
            selfHosted = state.selfHosted,
            selfHostedStyleTitle = state.selfHostedStyleTitle,
            customStyles = state.customStyles,
            activeCustomStyleId = state.activeCustomStyleId,
            onDismiss = { stylePickerVisible = false },
            onDisable = {
                stylePickerVisible = false
                onToggle(false)
            },
            onCustomStyle = onCustomStyle,
            onSelfHostedGeneral = {
                stylePickerVisible = false
                onSelfHostedGeneral()
            },
            onSelected = {
                stylePickerVisible = false
                if (it == TextStyle.CUSTOM) {
                    editedCustomStyleId = state.activeCustomStyleId
                    customEditorVisible = true
                } else {
                    onStyle(it)
                }
            },
        )
    }
    if (customEditorVisible && !state.selfHosted) {
        CustomInstructionDialog(
            preset = state.customStyles.firstOrNull { it.id == editedCustomStyleId },
            unlimitedInstruction = state.selfHosted,
            onDismiss = { customEditorVisible = false },
            onSave = { id, name, instruction, options ->
                onSaveCustomStyle(id, name, instruction, options)
                customEditorVisible = false
            },
            onDelete = editedCustomStyleId?.let { id ->
                { onDeleteCustomStyle(id); customEditorVisible = false }
            },
        )
    }
    state.notice?.let { notice ->
        AlertDialog(
            onDismissRequest = onDismissNotice,
            title = { Text(stringResource(R.string.stylization_unavailable_title)) },
            text = { Text(notice) },
            confirmButton = {
                TextButton(onClick = onDismissNotice) {
                    Text(stringResource(R.string.stylization_unavailable_close))
                }
            },
        )
    }
    if (emojiPickerVisible && !state.selfHosted) {
        ProductChoiceSheet(
            title = stringResource(R.string.stylization_emoji_title),
            selected = state.emojiMode,
            options = EmojiMode.entries,
            optionLabel = ::emojiModeOptionLabel,
            onSelected = onEmojiModeChange,
            onDismissRequest = { emojiPickerVisible = false },
        )
    }
}

internal fun emojiModeTitle(mode: EmojiMode): String = when (mode) {
    EmojiMode.OFF -> "Выключены"
    EmojiMode.RESTRAINED -> "Сдержанно"
    EmojiMode.LIVELY -> "Живо"
    EmojiMode.BRIGHT -> "Ярко"
}

internal fun emojiModeSummary(mode: EmojiMode): String = when (mode) {
    EmojiMode.OFF -> "Автоматические эмодзи не добавляются."
    EmojiMode.RESTRAINED -> "Только явно выраженные эмоции: лица и сердца."
    EmojiMode.LIVELY -> "Эмоции, реакции и жесты без тематических картинок."
    EmojiMode.BRIGHT -> "Эмоции и связанные с содержанием тематические значки."
}

internal fun emojiModeOptionLabel(mode: EmojiMode): String = when (mode) {
    EmojiMode.OFF -> "Выключены"
    EmojiMode.RESTRAINED -> "Сдержанно · лица и сердца"
    EmojiMode.LIVELY -> "Живо · эмоции и реакции"
    EmojiMode.BRIGHT -> "Ярко · включая тематические"
}

internal fun showProcessingUnavailable(state: StylizationUiState): Boolean =
    !state.selfHosted && state.hasAccess && state.enabled && state.termsAccepted &&
        !state.processingReady && state.processingMessage != null
