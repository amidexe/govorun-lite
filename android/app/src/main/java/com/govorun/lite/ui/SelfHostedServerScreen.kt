package com.govorun.lite.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import com.govorun.lite.stylization.SelfHostedConsent
import com.govorun.lite.stylization.SelfHostedServerSettings
import com.product.ui.*

@Composable
internal fun SelfHostedServerScreen(
    settings: SelfHostedServerSettings,
    connected: Boolean,
    accepted: Boolean,
    testing: Boolean,
    status: String,
    onChange: (SelfHostedServerSettings) -> Unit,
    onConsent: (Boolean) -> Unit,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
    onDeleteKey: () -> Unit,
    onClear: () -> Unit,
    onCancel: () -> Unit,
    onBack: () -> Unit,
) {
    var confirmClear by remember { mutableStateOf(false) }
    if (confirmClear) AlertDialog(
        onDismissRequest = { confirmClear = false },
        title = { Text("Сбросить настройки сервера?") },
        text = { Text("Настройки подключения и сохранённые API-ключи будут очищены на этом устройстве. " +
            "Обработка своим сервером отключится.") },
        confirmButton = { TextButton(onClick = { confirmClear = false; onClear() }) { Text("Сбросить") } },
        dismissButton = { TextButton(onClick = { confirmClear = false }) { Text("Отмена") } },
    )
    ProductScaffold(title = "Свой сервер", onBack = onBack) { innerPadding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            contentPadding = productScreenPadding(),
            verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md),
        ) {
            item {
                SettingsSection("Подключение") {
                    Column(Modifier.padding(ProductTokens.Spacing.Section),
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Md)) {
                        Text("OpenAI-совместимый API · ключ, оплату и доступ к серверу вы настраиваете сами",
                            style = MaterialTheme.typography.bodySmall)
                        OutlinedTextField(settings.endpoint, { onChange(settings.copy(endpoint = it)) },
                            Modifier.fillMaxWidth(), enabled = !testing,
                            label = { Text("Адрес сервера") },
                            placeholder = { Text("https://server.example/v1") }, singleLine = true)
                        OutlinedTextField(settings.model,
                            { onChange(settings.copy(model = it.take(SelfHostedServerSettings.MAX_MODEL_LENGTH))) },
                            Modifier.fillMaxWidth(), enabled = !testing,
                            label = { Text("Модель") }, singleLine = true)
                        OutlinedTextField(settings.apiKey,
                            { onChange(settings.copy(apiKey = it.take(SelfHostedServerSettings.MAX_API_KEY_LENGTH))) },
                            Modifier.fillMaxWidth(), enabled = !testing,
                            label = { Text("API-ключ, если требуется") },
                            visualTransformation = PasswordVisualTransformation(), singleLine = true)
                        TextButton(onDeleteKey, enabled = !testing && settings.apiKey.isNotEmpty()) {
                            Text("Удалить сохранённый ключ")
                        }
                        OutlinedTextField(settings.generalPrompt,
                            { onChange(settings.copy(generalPrompt = it)) },
                            Modifier.fillMaxWidth(), enabled = !testing,
                            label = { Text("Промпт — один стиль для всех диктовок") }, minLines = 3, maxLines = 6)
                    }
                }
            }
            item {
                SettingsSection("Прокси, если требуется") {
                    Column(Modifier.padding(ProductTokens.Spacing.Section)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Switch(settings.proxyEnabled, { onChange(settings.copy(proxyEnabled = it)) }, enabled = !testing)
                            Text("Использовать прокси", Modifier.padding(start = ProductTokens.Spacing.Sm))
                        }
                        if (settings.proxyEnabled) {
                            OutlinedTextField(settings.proxyURL, { onChange(settings.copy(proxyURL = it)) },
                                Modifier.fillMaxWidth(), enabled = !testing, singleLine = true,
                                label = { Text("Адрес прокси") }, placeholder = { Text("http://host:8080 или socks5://host:1080") })
                            Text("HTTP или SOCKS5 без логина и пароля. Прокси также участвует в передаче данных.",
                                style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
            item {
                SettingsSection("Передача данных") {
                    Column(Modifier.padding(ProductTokens.Spacing.Section),
                        verticalArrangement = Arrangement.spacedBy(ProductTokens.Spacing.Sm)) {
                        Text(SelfHostedConsent.NOTICE, style = MaterialTheme.typography.bodySmall)
                        if (settings.endpoint.trim().startsWith("http://", ignoreCase = true)) {
                            Text("HTTP не шифрует текст и ключ при передаче.",
                                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Checkbox(accepted, onConsent, enabled = !testing)
                            Text(SelfHostedConsent.ACKNOWLEDGEMENT, style = MaterialTheme.typography.bodyMedium)
                        }
                        if (status.isNotEmpty()) Text(status, style = MaterialTheme.typography.bodyMedium)
                        Button(onConnect, enabled = accepted && settings.isComplete && !testing,
                            modifier = Modifier.fillMaxWidth()) {
                            Text(if (testing) "Проверяем…" else if (connected && settings.isComplete) "Применить и проверить" else "Подключить")
                        }
                        if (testing) TextButton(onCancel) { Text("Отменить проверку") }
                        if (connected) TextButton(onDisconnect, enabled = !testing) { Text("Отключить свой сервер") }
                    }
                }
            }
            item {
                TextButton(onClick = { confirmClear = true }, enabled = !testing) {
                    Text("Сбросить настройки сервера", color = MaterialTheme.colorScheme.error)
                }
            }
        }
    }
}
