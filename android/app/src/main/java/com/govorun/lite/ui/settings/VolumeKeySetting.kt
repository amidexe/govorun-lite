package com.govorun.lite.ui.settings

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue

/**
 * «Диктовка по удержанию «−»» (an experiment, off by default). Turning it on asks once, plainly, because it changes what a
 * volume key does: the person should think whether they need it. Turning it off needs no question.
 */
@Composable
internal fun VolumeKeySetting(
    checked: Boolean,
    onChange: (Boolean) -> Unit,
) {
    var asking by rememberSaveable { mutableStateOf(false) }
    SettingsSwitchRow(
        title = "Диктовка по удержанию «−» (эксперимент)",
        summary = "Где видна птичка, удержание кнопки громкости «−» начинает диктовку, как удержание самой птички",
        checked = checked,
        onCheckedChange = { if (it) asking = true else onChange(false) },
    )
    if (asking) {
        AlertDialog(
            onDismissRequest = { asking = false },
            title = { Text("Включить диктовку по «−»?") },
            text = {
                Text(
                    "Когда видна птичка, удержание «−» — это диктовка, а громкость в этот момент меняется только коротким " +
                        "нажатием. Во время звонка, будильника и музыки кнопка остаётся обычной громкостью.\n\n" +
                        "Это эксперимент: если мешает, выключите здесь же.",
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    asking = false
                    onChange(true)
                }) { Text("Включить") }
            },
            dismissButton = { TextButton(onClick = { asking = false }) { Text("Отмена") } },
        )
    }
}
