package com.govorun.lite.ui.settings

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue

@Composable
internal fun ServiceControlSettings(
    serviceEnabled: Boolean,
    onEvent: (SettingsEvent) -> Unit,
) {
    var confirmDisable by rememberSaveable(serviceEnabled) { mutableStateOf(false) }

    SettingsSection("Работа Говоруна") {
        SettingsSwitchRow(
            title = if (serviceEnabled) "Говорун включён" else "Говорун выключен",
            summary = if (serviceEnabled) {
                "Птичка появляется над клавиатурой"
            } else {
                "Для включения откроются специальные возможности Android"
            },
            checked = serviceEnabled,
            onCheckedChange = { enabled ->
                if (enabled) {
                    onEvent(SettingsEvent.OpenAccessibility)
                } else {
                    confirmDisable = true
                }
            },
        )
    }

    if (confirmDisable && serviceEnabled) {
        AlertDialog(
            onDismissRequest = { confirmDisable = false },
            title = { Text("Выключить Говорун?") },
            text = {
                Text(
                    "Птичка исчезнет, диктовка остановится. " +
                        "Чтобы включить Говорун снова, придётся заново разрешить " +
                        "ему доступ в специальных возможностях Android. " +
                        "Настройки приложения сохранятся.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        if (confirmDisable) {
                            confirmDisable = false
                            onEvent(SettingsEvent.ConfirmDisableService)
                        }
                    },
                ) { Text("Выключить") }
            },
            dismissButton = {
                TextButton(onClick = { confirmDisable = false }) { Text("Отмена") }
            },
        )
    }
}
