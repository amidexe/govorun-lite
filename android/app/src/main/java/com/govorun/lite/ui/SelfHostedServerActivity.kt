package com.govorun.lite.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StylizationInputMode
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.*
import com.govorun.lite.ui.design.GovorunTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

class SelfHostedServerActivity : ComponentActivity() {
    private var settings by mutableStateOf(SelfHostedServerSettings())
    private var connected by mutableStateOf(false)
    private var accepted by mutableStateOf(false)
    private var status by mutableStateOf("")
    private var testing by mutableStateOf(false)
    private var connectionJob: Job? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        settings = StylizationServerPreferences.settings(this)
        accepted = settings.hasConsent
        connected = StylizationServerPreferences.mode(this) == StylizationServerMode.SELF_HOSTED
        status = if (connected && !accepted) "Подтвердите передачу данных, чтобы продолжить" else ""
        setContent {
            GovorunTheme {
                SelfHostedServerScreen(settings, connected, accepted, testing, status,
                    onChange = { next ->
                        if (next.recipient != settings.recipient) {
                            accepted = false
                            StylizationServerPreferences.revokeConsent(this)
                        }
                        settings = next
                        status = ""
                    },
                    onConsent = {
                        accepted = it
                        if (!it) StylizationServerPreferences.revokeConsent(this)
                    },
                    onConnect = ::connect,
                    onDisconnect = ::disconnect,
                    onDeleteKey = {
                        if (StylizationServerPreferences.deleteApiKey(this)) {
                            settings = settings.copy(apiKey = "")
                            status = "Ключ удалён · обработка отключена"
                        } else status = "Не удалось удалить ключ"
                    },
                    onClear = {
                        if (StylizationServerPreferences.clearServer(this)) {
                            settings = StylizationServerPreferences.settings(this)
                            accepted = false
                            connected = StylizationServerPreferences.mode(this) == StylizationServerMode.SELF_HOSTED
                            status = "Настройки сервера сброшены · обработка отключена"
                        } else status = "Не удалось сохранить сброс на устройстве. Повторите попытку."
                    },
                    onCancel = { connectionJob?.cancel() },
                    onBack = ::finish,
                )
            }
        }
    }

    private fun connect() {
        if (!accepted || !settings.isComplete || testing) return
        val candidate = settings.copy(acceptedRecipient = settings.recipient.orEmpty())
        if (!StylizationServerPreferences.saveSettings(this, candidate)) {
            status = "Не удалось сохранить настройки и ключ"
            return
        }
        // Select the explicit route before checking: a failure must never fall through to Govorun.
        StylizationServerPreferences.setMode(this, StylizationServerMode.SELF_HOSTED)
        StylizationServerPreferences.setStylizationEnabled(this, false)
        connected = true
        testing = true
        status = "Проверяем сервер…"
        connectionJob = lifecycleScope.launch {
            val started = android.os.SystemClock.elapsedRealtime()
            try {
                OpenAiCompatibleStylizationClient(candidate).stylize(
                    text = "это проверка подключения завтра встречаемся в 15:30",
                    style = TextStyle.CUSTOM, inputMode = StylizationInputMode.CONFIRM,
                    instruction = candidate.generalPrompt, punctuationMode = PunctuationMode.NORMAL,
                )
                StylizationServerPreferences.markVerified(this@SelfHostedServerActivity)
                StylizationServerPreferences.selectGeneral(this@SelfHostedServerActivity)
                StylizationServerPreferences.setStylizationEnabled(this@SelfHostedServerActivity, true)
                settings = candidate
                status = "Сервер работает · обработка включена"
                record("success", started)
            } catch (error: CancellationException) {
                status = "Проверка отменена · обработка отключена"
                record("cancelled", started)
                throw error
            } catch (error: Exception) {
                val auth = error is StylizationHttpException && error.statusCode in setOf(401, 403)
                status = if (auth) "Сервер отклонил ключ. Проверьте API-ключ и доступ к модели"
                    else "Не удалось подключиться. Проверьте адрес, модель и ключ"
                record(if (auth) "auth_failed" else "failed", started)
            } finally {
                testing = false
            }
        }
    }

    private fun record(result: String, started: Long) {
        LocalDiagnosticJournal.recordTiming(this,
            SelfHostedDiagnostic(result, android.os.SystemClock.elapsedRealtime() - started).encode())
    }

    private fun disconnect() {
        connectionJob?.cancel()
        StylizationServerPreferences.setStylizationEnabled(this, false)
        StylizationServerPreferences.setMode(this, StylizationServerMode.GOVORUN)
        connected = false
        status = "Свой сервер отключён. Настройки сохранены"
    }
}
