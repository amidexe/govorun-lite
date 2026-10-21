package com.govorun.lite.ui

import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.govorun.lite.settings.SettingsController
import com.govorun.lite.settings.AndroidSettingsModule
import com.govorun.lite.R
import com.govorun.lite.api.AuthorizedAppsActivity
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.service.LiteAccessibilityService
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.home.HomeSnapshotReader
import com.govorun.lite.ui.settings.SettingsEvent
import com.govorun.lite.ui.settings.SettingsScreen
import com.govorun.lite.ui.settings.SettingsSnapshotReader
import com.govorun.lite.ui.settings.SettingsUiState
import com.govorun.lite.util.AccessibilityHelper
import kotlinx.coroutines.launch
import com.govorun.lite.stylization.diagnosticOrElse

/**
 * Compose shell: renders snapshots and handles Android navigation.
 * Preference rules and persistence are composed at the application boundary.
 */
class SettingsActivity : ComponentActivity() {

    private lateinit var settings: SettingsController
    private lateinit var snapshotReader: SettingsSnapshotReader
    private var uiState by mutableStateOf(SettingsUiState.loading())

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        snapshotReader = SettingsSnapshotReader(this)
        settings = AndroidSettingsModule.create(this)
        setContent {
            GovorunTheme {
                SettingsScreen(uiState, ::handleEvent)
            }
        }
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch { GovorunAccountRepository.state.collect { refresh() } }
                launch { com.govorun.lite.stylization.LocalDiagnosticJournal.changes.collect { refresh() } }
                launch { com.govorun.lite.diagnostics.AppLifecycleDiagnostics.serviceConnected.collect { refresh() } }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        GovorunAccountRepository.refreshAccount()
        refresh()
    }

    private fun refresh() {
        uiState = snapshotReader.read()
    }

    private fun handleEvent(event: SettingsEvent) {
        when (event) {
            SettingsEvent.Back -> finish()
            is SettingsEvent.Change -> {
                if (settings.apply(event.value).stylizationDisabled) {
                    Toast.makeText(
                        this,
                        R.string.recording_mode_after_pause_disables_styles,
                        Toast.LENGTH_SHORT,
                    ).show()
                }
            }
            SettingsEvent.OpenStylization -> openOnlineFeature(StylizationActivity::class.java)
            SettingsEvent.OpenSelfHostedServer -> open(SelfHostedServerActivity::class.java)
            SettingsEvent.OpenSubscription -> open(SubscriptionActivity::class.java)
            SettingsEvent.OpenDictionary -> open(DictionaryActivity::class.java)
            SettingsEvent.OpenAppFilter -> open(AppFilterActivity::class.java)
            SettingsEvent.OpenBenchmark -> open(BenchmarkActivity::class.java)
            SettingsEvent.OpenAuthorizedApps -> open(AuthorizedAppsActivity::class.java)
            SettingsEvent.OpenAccessibility ->
                AccessibilityHelper.openAccessibilitySettings(this)
            SettingsEvent.OpenJournal -> Unit // Nested navigation is owned by SettingsScreen.
            SettingsEvent.ConfirmDisableService -> {
                // A dead service cannot disable itself; send the user to the switch.
                LiteAccessibilityService.instance?.disableSelf()
                    ?: AccessibilityHelper.openAccessibilitySettings(this)
                window.decorView.postDelayed(::refresh, 350)
            }
            SettingsEvent.ClearDiagnostics ->
                com.govorun.lite.stylization.StylizationDiagnosticsStore.clearLocalHistory(this)
            SettingsEvent.CopyDiagnostics -> {
                val copied = diagnosticOrElse(false) {
                    val text = com.govorun.lite.ui.settings.journalText(
                        com.govorun.lite.stylization.LocalDiagnosticJournal.read(this),
                    )
                    val clipboard = getSystemService(android.content.ClipboardManager::class.java)
                    if (clipboard == null) false else {
                        clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Журнал Говоруна", text))
                        true
                    }
                }
                if (!copied || android.os.Build.VERSION.SDK_INT < 33) {
                    Toast.makeText(this, if (copied) "Журнал скопирован" else "Не удалось скопировать журнал",
                        Toast.LENGTH_SHORT).show()
                }
            }
            SettingsEvent.ResetStatistics -> {
                GovorunAccountRepository.requestSynchronizedStatsReset(this)
            }
            SettingsEvent.Reset -> settings.reset()
            SettingsEvent.ResetAppearance -> settings.resetAppearance()
        }
        if (event.updatesSnapshot) refresh()
    }

    private fun open(destination: Class<*>) {
        startActivity(Intent(this, destination))
    }

    private fun openOnlineFeature(destination: Class<*>) {
        open(
            if (HomeSnapshotReader(this).read(justFinished = false).ready) destination
            else SubscriptionActivity::class.java,
        )
    }

}
