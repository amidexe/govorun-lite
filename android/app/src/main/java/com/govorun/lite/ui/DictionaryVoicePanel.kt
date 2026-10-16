package com.govorun.lite.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.adamglin.PhosphorIcons
import com.adamglin.phosphoricons.Regular
import com.adamglin.phosphoricons.regular.Microphone
import com.adamglin.phosphoricons.regular.Stop
import com.adamglin.phosphoricons.regular.X
import com.govorun.lite.R
import com.product.ui.ProductDivider
import com.product.ui.ProductScaffold
import com.product.ui.ProductTokens
import com.product.ui.ProductTopAction

internal data class DictionaryVoiceCaptureState(
    val visible: Boolean = false,
    val correctText: String = "",
    val variants: List<String> = emptyList(),
    val recording: Boolean = false,
    val processing: Boolean = false,
    val error: String? = null,
) {
    val busy: Boolean get() = recording || processing
    val canApply: Boolean get() = correctText.isNotBlank() && variants.isNotEmpty() && !busy
}

/** Full-screen capture: only the results scroll; the field and actions stay in view. */
@Composable
internal fun DictionaryVoicePanel(
    state: DictionaryVoiceCaptureState,
    onCorrectTextChange: (String) -> Unit,
    onRecord: () -> Unit,
    onStop: () -> Unit,
    onRemoveVariant: (Int) -> Unit,
    onApply: () -> Unit,
    onDismiss: () -> Unit,
) {
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    val listState = rememberLazyListState()
    var previousCount by remember { mutableIntStateOf(0) }
    val removeDescription = stringResource(R.string.dict_voice_remove)
    fun hideKeyboard() {
        focusManager.clearFocus()
        keyboard?.hide()
    }

    LaunchedEffect(Unit) { hideKeyboard() }
    // Also cover permission return and the last result arriving after Stop.
    LaunchedEffect(state.busy) {
        if (state.busy) hideKeyboard()
    }
    LaunchedEffect(state.variants.size) {
        if (state.variants.size > previousCount) {
            listState.animateScrollToItem(state.variants.lastIndex)
        }
        previousCount = state.variants.size
    }

    ProductScaffold(
        title = stringResource(R.string.dict_voice_title),
        actions = listOf(
            ProductTopAction(PhosphorIcons.Regular.X, stringResource(R.string.dict_voice_cancel)) {
                hideKeyboard()
                onDismiss()
            },
        ),
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .consumeWindowInsets(innerPadding)
                .imePadding()
                .padding(horizontal = ProductTokens.Spacing.Section)
                .testTag("dictionary_voice_panel"),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            OutlinedTextField(
                value = state.correctText,
                onValueChange = onCorrectTextChange,
                modifier = Modifier.fillMaxWidth().testTag("dictionary_voice_correct"),
                label = { Text(stringResource(R.string.dict_voice_correct)) },
                enabled = !state.busy,
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { hideKeyboard() }),
            )
            Text(
                text = if (state.variants.isEmpty()) stringResource(R.string.dict_voice_instruction)
                    else stringResource(R.string.dict_voice_variants, state.variants.size),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxWidth().testTag("dictionary_voice_variants"),
            ) {
                itemsIndexed(state.variants, key = { _, variant -> variant }) { index, variant ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            variant,
                            modifier = Modifier.weight(1f).padding(vertical = ProductTokens.Spacing.Md),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                        IconButton(
                            onClick = { onRemoveVariant(index) },
                            enabled = !state.busy,
                        ) {
                            Icon(
                                PhosphorIcons.Regular.X,
                                contentDescription = "$removeDescription: $variant",
                                modifier = Modifier.size(ProductTokens.Size.ActiveIcon),
                            )
                        }
                    }
                    ProductDivider()
                }
            }
            state.error?.let {
                Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.error)
            }
            BoxWithConstraints(
                modifier = Modifier.fillMaxWidth().padding(bottom = ProductTokens.Spacing.Section),
            ) {
                FlowRow(
                    modifier = Modifier.fillMaxWidth(),
                    maxItemsInEachRow = if (maxWidth / LocalDensity.current.fontScale < 340.dp) 1 else 2,
                    horizontalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
                    verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm),
                ) {
                    VoiceRecordButton(
                        state = state,
                        onClick = {
                            hideKeyboard()
                            if (state.recording) onStop() else onRecord()
                        },
                        modifier = Modifier.weight(1f),
                    )
                    Button(
                        onClick = { hideKeyboard(); onApply() },
                        enabled = state.canApply,
                        modifier = Modifier.weight(1f).testTag("dictionary_voice_apply"),
                    ) {
                        Text(stringResource(R.string.dict_voice_add))
                    }
                }
            }
        }
    }
}

@Composable
private fun VoiceRecordButton(
    state: DictionaryVoiceCaptureState,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    FilledTonalButton(
        onClick = onClick,
        modifier = modifier.testTag("dictionary_voice_record"),
        enabled = !state.processing,
    ) {
        if (state.processing) {
            CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
        } else {
            Icon(
                if (state.recording) PhosphorIcons.Regular.Stop else PhosphorIcons.Regular.Microphone,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
            )
        }
        Spacer(Modifier.width(ProductTokens.Spacing.Sm))
        Text(stringResource(when {
            state.processing -> R.string.dict_voice_processing
            state.recording -> R.string.dict_voice_stop
            state.variants.isNotEmpty() -> R.string.dict_voice_resume
            else -> R.string.dict_voice_start
        }))
    }
}
