package com.govorun.lite.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Duotone
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.duotone.Info
import com.adamglin.phosphoricons.regular.Microphone
import com.govorun.lite.R
import com.govorun.lite.transcriber.DictionaryRuleEditor
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.ProductTopAction
import com.product.ui.SettingsSection
import com.product.ui.SettingsSwitchRow
import com.product.ui.productScreenPadding

@Composable
internal fun DictionaryScreen(
    initialText: String,
    initialSavedText: String = initialText,
    initialEnabled: Boolean,
    initialShowOnHome: Boolean,
    voiceState: DictionaryVoiceCaptureState,
    onBack: () -> Unit,
    onEnabled: (Boolean) -> Unit,
    onShowOnHome: (Boolean) -> Unit,
    onSave: (String) -> Unit,
    onSaveVoice: (String) -> Boolean,
    onOpenVoice: () -> Unit,
    onVoiceCorrectText: (String) -> Unit,
    onVoiceRecord: () -> Unit,
    onVoiceStop: () -> Unit,
    onVoiceRemove: (Int) -> Unit,
    onVoiceDismiss: () -> Unit,
) {
    var value by rememberSaveable { mutableStateOf(initialText) }
    var savedValue by rememberSaveable { mutableStateOf(initialSavedText) }
    var enabled by rememberSaveable { mutableStateOf(initialEnabled) }
    var showOnHome by rememberSaveable { mutableStateOf(initialShowOnHome) }
    var helpVisible by rememberSaveable { mutableStateOf(false) }
    val context = androidx.compose.ui.platform.LocalContext.current
    if (voiceState.visible) {
        BackHandler(onBack = onVoiceDismiss)
        DictionaryVoicePanel(
            state = voiceState,
            onCorrectTextChange = onVoiceCorrectText,
            onRecord = onVoiceRecord,
            onStop = onVoiceStop,
            onRemoveVariant = onVoiceRemove,
            onApply = {
                try {
                    val nextSaved = DictionaryRuleEditor.upsertVoiceMappings(
                        savedValue, voiceState.correctText, voiceState.variants,
                    )
                    val nextDraft = DictionaryRuleEditor.upsertVoiceMappings(
                        value, voiceState.correctText, voiceState.variants,
                    )
                    if (onSaveVoice(nextSaved)) {
                        savedValue = nextSaved
                        value = nextDraft
                        onVoiceDismiss()
                    }
                } catch (_: Exception) {
                    android.widget.Toast.makeText(context, com.govorun.lite.R.string.dict_invalid,
                        android.widget.Toast.LENGTH_LONG).show()
                }
            },
            onDismiss = onVoiceDismiss,
        )
        return
    }

    ProductScaffold(
        title = "Словарь и автозамены",
        onBack = onBack,
        actions = listOf(
            ProductTopAction(PhosphorIcons.Duotone.Info, "Как это работает") {
                helpVisible = true
            },
        ),
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .imePadding(),
        ) {
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = productScreenPadding(bottom = ProductTokens.Spacing.Md),
                verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
            ) {
                item {
                    SettingsSection("После распознавания") {
                        SettingsSwitchRow(
                            title = "Применять автозамены",
                            summary = "Исправлять имена, термины и другие указанные слова",
                            checked = enabled,
                            onCheckedChange = {
                                enabled = it
                                onEnabled(it)
                            },
                        )
                        ProductDivider()
                        SettingsSwitchRow(
                            title = "Показывать на главном экране",
                            summary = "Быстрый переход к словарю",
                            checked = showOnHome,
                            onCheckedChange = {
                                showOnHome = it
                                onShowOnHome(it)
                            },
                        )
                        ProductDivider()
                        Text(
                            "После входа словарь автоматически синхронизируется между вашими устройствами.",
                            modifier = Modifier.padding(ProductTokens.Spacing.Section),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                item {
                    FilledTonalButton(
                        onClick = onOpenVoice,
                        modifier = Modifier.fillMaxWidth().testTag("dictionary_add_voice"),
                    ) {
                        Icon(PhosphorIcons.Regular.Microphone, contentDescription = null)
                        Text(
                            stringResource(R.string.dict_voice_title),
                            modifier = Modifier.padding(start = ProductTokens.Spacing.Sm),
                        )
                    }
                }
                item {
                    SettingsSection("Все правила") {
                        OutlinedTextField(
                            value = value,
                            onValueChange = { value = it },
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(ProductTokens.Spacing.Section)
                                .testTag("dictionary_editor"),
                            minLines = 6,
                            maxLines = 12,
                            textStyle = MaterialTheme.typography.bodyMedium,
                            label = { Text("Одна замена на строку") },
                        )
                        Button(
                            onClick = { onSave(value) },
                            enabled = value != savedValue,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(
                                    start = ProductTokens.Spacing.Section,
                                    end = ProductTokens.Spacing.Section,
                                    bottom = ProductTokens.Spacing.Section,
                                )
                                .testTag("dictionary_save"),
                        ) {
                            Text("Сохранить правила")
                        }
                    }
                }
            }
        }
    }
    if (helpVisible) {
        AlertDialog(
            onDismissRequest = { helpVisible = false },
            title = { Text(stringResource(R.string.dict_help_dialog_title)) },
            text = { Text(stringResource(R.string.dict_help_dialog_body)) },
            confirmButton = {
                TextButton(onClick = { helpVisible = false }) {
                    Text(stringResource(R.string.dict_help_dialog_close))
                }
            },
        )
    }
}
