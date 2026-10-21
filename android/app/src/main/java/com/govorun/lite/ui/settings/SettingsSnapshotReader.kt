package com.govorun.lite.ui.settings

import android.content.Context
import com.govorun.lite.BuildConfig
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stylization.titleRes
import com.govorun.lite.util.AccessibilityHelper
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import com.govorun.lite.util.StylizationPrefs
import com.govorun.lite.util.UiAppearancePrefs
import com.govorun.lite.util.VoiceMarkerPrefs

/**
 * Read-only bridge to the existing 1.x stores. No settings are renamed or
 * copied, so an in-place RuStore update sees exactly the user's prior values.
 */
class SettingsSnapshotReader(context: Context) {
    private val appContext = context.applicationContext

    fun read(): SettingsUiState {
        val appearance = UiAppearancePrefs.read(appContext)
        val account = GovorunAccountRepository.state.value
        return SettingsUiState(
            selfHosted = com.govorun.lite.stylization.StylizationServerPreferences.mode(appContext) ==
                com.govorun.lite.stylization.StylizationServerMode.SELF_HOSTED,
            themeMode = appearance.themeMode,
            dynamicColor = appearance.dynamicColor,
            bubbleSide = Prefs.getBubbleSide(appContext),
            bubbleSize = Prefs.getBubbleSize(appContext),
            bubbleAlpha = Prefs.getBubbleAlpha(appContext),
            edgeMargin = Prefs.getBubbleEdgeMargin(appContext),
            recordingAlpha = Prefs.getBubbleRecordingAlpha(appContext),
            birdColor = Prefs.getBirdColor(appContext),
            edgeMarginMax = Prefs.bubbleEdgeMarginMax(appContext),
            edgeMarginCustomized = com.govorun.lite.overlay.BubblePlacement.hasCustomMargin(appContext),
            recordingMode = RecordingModePrefs.read(appContext),
            haptics = Prefs.isHapticsEnabled(appContext),
            positionLocked = Prefs.isBubblePositionLocked(appContext),
            pause = Prefs.getPauseLength(appContext),
            stylizationSummary = stylizationSummary(),
            accessSummary = settingsAccessSummary(account),
            voiceMarkerMode = VoiceMarkerPrefs.read(appContext),
            legacyVoiceMarker = VoiceMarkerPrefs.deliveryMarker(appContext) == "legacy_text",
            hideInSearch = Prefs.isHideInSearchEnabled(appContext),
            appFilterSummary = appFilterSummary(),
            keepScreenOn = Prefs.isKeepScreenOnEnabled(appContext),
            rememberLastDictation = com.govorun.lite.service.LastDictation.isEnabled(appContext),
            autoStopMinutes = Prefs.getAutoStopMinutes(appContext),
            serviceEnabled = AccessibilityHelper.isLiteServiceEnabled(appContext),
            diagnosticEvents = localDiagnosticRows(appContext),
            birdConnectionNeedsAttention = birdConnectionNeedsAttention(
                AccessibilityHelper.isLiteServiceEnabled(appContext),
                com.govorun.lite.diagnostics.AppLifecycleDiagnostics.serviceConnected.value,
            ),
        )
    }

    private fun stylizationSummary(): String = when {
        !hasStylizationService() ->
            appContext.getString(R.string.settings_stylization_body_unavailable)
        !Prefs.isStylizationEnabled(appContext) ->
            appContext.getString(R.string.settings_stylization_body_off)
        else -> {
            val style = TextStyle.fromApiId(Prefs.getStylizationStyle(appContext))
            appContext.getString(
                R.string.settings_stylization_body_on,
                appContext.getString(style.titleRes),
            )
        }
    }

    private fun hasStylizationService(): Boolean =
        BuildConfig.ACCOUNT_API_URL.startsWith("https://") &&
            BuildConfig.OFFICIAL_STYLES_API_URL.startsWith("https://")

    private fun appFilterSummary(): String {
        val mode = Prefs.getAppFilterMode(appContext)
        val count = Prefs.getAppFilterPackages(appContext).size
        return when {
            count == 0 -> appContext.getString(R.string.settings_app_filter_body_all)
            mode == Prefs.APP_FILTER_WHITELIST ->
                appContext.getString(R.string.settings_app_filter_body_whitelist, count)
            else -> appContext.getString(R.string.settings_app_filter_body_blacklist, count)
        }
    }
}
