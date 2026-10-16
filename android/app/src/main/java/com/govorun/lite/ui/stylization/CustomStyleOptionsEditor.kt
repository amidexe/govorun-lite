package com.govorun.lite.ui.stylization

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.res.stringResource
import com.govorun.lite.R
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.product.ui.ProductChoiceSheet
import com.product.ui.ProductDivider
import com.product.ui.SettingsLinkRow
import com.product.ui.SettingsSwitchRow

@Composable
internal fun CustomStyleOptionsEditor(options: StyleOptions, onChange: (StyleOptions) -> Unit) {
    var emojiPickerVisible by rememberSaveable { mutableStateOf(false) }
    Text("Параметры этого стиля", style = MaterialTheme.typography.labelLarge)
    SettingsLinkRow(
        title = stringResource(R.string.stylization_emoji_title),
        summary = emojiModeSummary(options.resolvedEmojiMode),
        value = emojiModeTitle(options.resolvedEmojiMode),
        onClick = { emojiPickerVisible = true },
    )
    ProductDivider()
    SettingsSwitchRow(
        title = stringResource(R.string.stylization_reduced_punctuation_title),
        summary = stringResource(R.string.stylization_reduced_punctuation_body),
        checked = options.punctuation == PunctuationMode.LESS,
        onCheckedChange = { onChange(options.copy(punctuation = if (it) PunctuationMode.LESS else PunctuationMode.NORMAL)) },
    )
    ProductDivider()
    SettingsSwitchRow(
        title = stringResource(R.string.stylization_profanity_filter_title),
        summary = stringResource(R.string.stylization_profanity_filter_body),
        checked = options.profanityFilterEnabled,
        onCheckedChange = { onChange(options.copy(profanityFilterEnabled = it)) },
    )
    if (emojiPickerVisible) ProductChoiceSheet(
        title = stringResource(R.string.stylization_emoji_title),
        selected = options.resolvedEmojiMode,
        options = EmojiMode.entries,
        optionLabel = ::emojiModeOptionLabel,
        onSelected = { onChange(options.withEmojiMode(it)) },
        onDismissRequest = { emojiPickerVisible = false },
    )
}
