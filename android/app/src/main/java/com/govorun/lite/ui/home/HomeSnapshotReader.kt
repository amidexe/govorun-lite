package com.govorun.lite.ui.home

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.PowerManager
import androidx.core.content.ContextCompat
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.officialStyleSelected
import com.govorun.lite.account.stylePauseMessage
import com.govorun.lite.account.connectionMessage
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.stats.StatsStore
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.SelfHostedStylePresets
import com.govorun.lite.stylization.StylizationServerMode
import com.govorun.lite.stylization.StylizationServerPreferences
import com.govorun.lite.stylization.StylizationStylePreferences
import com.govorun.lite.stylization.StylizationTerms
import com.govorun.lite.stylization.StyleProcessingFeedback
import com.govorun.lite.stylization.styleProcessingPresentation
import com.govorun.lite.diagnostics.AppLifecycleDiagnostics
import com.govorun.lite.util.AccessibilityHelper
import com.govorun.lite.util.HomePreferences
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.XiaomiShell
import java.time.LocalDate

/**
 * Android-backed snapshot boundary for the Compose home screen.
 *
 * It deliberately reuses the 1.x stores instead of introducing a second
 * preferences schema. The class only reads; mutations stay in MainActivity.
 */
class HomeSnapshotReader(context: Context) {
    private val appContext = context.applicationContext

    fun read(justFinished: Boolean): HomeUiState {
        val microphoneReady = ContextCompat.checkSelfPermission(
            appContext,
            Manifest.permission.RECORD_AUDIO,
        ) == PackageManager.PERMISSION_GRANTED
        val accessibilityEnabled = AccessibilityHelper.isLiteServiceEnabled(appContext)
        // The in-process connection flag is the only reliable sign of a live bird:
        // MIUI/HyperOS keep the service switched on after unloading its process.
        val accessibilityReady = AccessibilityHelper.isLiteServiceOperational(appContext) &&
            AppLifecycleDiagnostics.serviceConnected.value
        val shortcutEnabled = AccessibilityHelper.isLiteShortcutEnabled(appContext)
        val batteryReady = (appContext.getSystemService(Context.POWER_SERVICE) as? PowerManager)
            ?.isIgnoringBatteryOptimizations(appContext.packageName) == true
        val ready = microphoneReady && accessibilityReady && !shortcutEnabled

        val issues = buildList {
            if (!microphoneReady) add(SetupIssue.MICROPHONE)
            if (!accessibilityEnabled) {
                add(SetupIssue.ACCESSIBILITY)
            } else if (!accessibilityReady) {
                add(SetupIssue.ACCESSIBILITY_RESTART)
                if (XiaomiShell.detected) add(SetupIssue.AUTOSTART)
            }
            if (accessibilityReady && shortcutEnabled) {
                add(SetupIssue.ACCESSIBILITY_SHORTCUT)
            }
            if (!batteryReady) add(SetupIssue.BATTERY)
        }
        val headline = when {
            justFinished && ready -> "Всё готово"
            ready -> "Говорите свободно"
            else -> "Завершите настройку"
        }
        val supporting = when {
            justFinished && ready -> "Говорун уже ждёт рядом с клавиатурой"
            ready -> "Текст появляется там, где стоит курсор"
            else -> "Исправьте пункты ниже — займёт меньше минуты"
        }

        val accountState = GovorunAccountRepository.state.value
        val hasStylizationConsent = Prefs.hasStylizationConsent(
            appContext,
            StylizationTerms.CURRENT_VERSION,
        )
        val selfHosted = StylizationServerPreferences.mode(appContext) ==
            StylizationServerMode.SELF_HOSTED
        val selfHostedSettings = StylizationServerPreferences.settings(appContext)
        val selfHostedEnabled = selfHosted && selfHostedSettings.isComplete &&
            StylizationServerPreferences.stylizationEnabled(appContext)
        val styleSelected = if (selfHosted) selfHostedEnabled else officialStyleSelected(
            Prefs.isStylizationEnabled(appContext), accountState,
        )
        val selectedStyleOptions = StylizationStylePreferences.optionsForSelected(appContext)
        val access = homeAccessPresentation(accountState)
        val connectionProblem = accountState.connectionMessage.takeIf { accountState.signedIn && !selfHosted }
        val accessVisible = selfHosted || accountState.available || access.active || styleSelected
        val termsRequired = requiresOnlineTermsAction(
            selfHosted = selfHosted,
            officialStyleSelected = styleSelected,
            localCurrentTermsAccepted = hasStylizationConsent,
            account = accountState,
        )
        val pauseMessage = if (!selfHosted && styleSelected && !termsRequired)
            accountState.stylePauseMessage else null
        val processing = accountState.styleProcessingPresentation()
        val feedback = StyleProcessingFeedback.snapshot(accountState.sessionGeneration, selfHosted)
        val feedbackMessage = if (feedback.phase == StyleProcessingFeedback.Phase.PROCESSING)
            "Улучшаем текст…" else feedback.failureMessage
        val offerStatus = when {
            !selfHosted && accountState.secureStorageProblem != null ->
                "Не удалось прочитать сохранённый вход"
            selfHosted && StylizationServerPreferences.isVerified(appContext) -> "Сервер работает"
            selfHosted && !selfHostedSettings.isComplete -> "Настройте свой сервер"
            selfHostedEnabled -> "Свой сервер · стилизация включена"
            selfHosted -> "Свой сервер настроен · стилизация отключена"
            connectionProblem != null -> connectionProblem
            termsRequired -> "Примите обновлённые условия"
            pauseMessage != null -> pauseMessage
            styleSelected -> processing.message ?: "Стили включены"
            access.active -> "Улучшение текста подключено"
            Prefs.getStylizationConsentVersion(appContext) in
                1 until StylizationTerms.CURRENT_VERSION ->
                "Примите обновлённые условия"
            accountState.available && !accountState.signedIn -> "Войдите, чтобы улучшать текст"
            accountState.hasOfficialAccess -> "Улучшение текста подключено"
            accountState.hasEntitlement -> "Примите условия интернет-обработки"
            accountState.signedIn -> "Для стилей нужен активный доступ"
            else -> "Проверяем доступность…"
        }
        return HomeUiState(
            ready = ready,
            headline = headline,
            supportingText = supporting,
            issues = issues,
            statistics = statisticsState(accountState.signedIn),
            lastDictation = com.govorun.lite.service.LastDictation.peek(appContext),
            lastDictationOriginal = com.govorun.lite.service.LastDictation.peekOriginal(appContext),
            dictionary = DictionarySummaryState(
                enabled = Prefs.isDictionaryEnabled(appContext),
                visible = HomePreferences.showDictionaryOnHome(appContext),
            ),
            stylization = StylizationOfferState(
                signedIn = accountState.signedIn,
                visible = shouldShowOnlineAccessOnHome(ready, accessVisible),
                starterAvailable = accountState.starterGrant.eligible,
                status = offerStatus,
                pauseMessage = pauseMessage,
                accessLabel = when {
                    selfHosted -> offerStatus
                    termsRequired -> "Чтобы улучшение текста снова работало"
                    else -> access.label
                },
                actionEnabled = if (selfHosted) selfHostedSettings.isComplete
                    else connectionProblem != null || termsRequired || accountState.available || styleSelected,
                retryConnection = connectionProblem != null,
                accessActive = selfHosted || access.active,
                enabled = styleSelected && !termsRequired && pauseMessage == null,
                processingReady = selfHostedEnabled || processing.ready && connectionProblem == null,
                processingMessage = if (styleSelected && !termsRequired && pauseMessage == null)
                    feedbackMessage ?: processing.message.takeIf { !selfHosted } else null,
                processingFailed = feedback.phase == StyleProcessingFeedback.Phase.FAILED,
                processingRevision = feedback.revision,
                termsRequired = termsRequired,
                canEnableDirectly = if (selfHosted) selfHostedSettings.isComplete && selfHostedSettings.hasConsent
                    else connectionProblem == null && accountState.hasOfficialAccess && hasStylizationConsent && pauseMessage == null &&
                        com.govorun.lite.util.RecordingModePrefs.read(appContext) ==
                            com.govorun.lite.util.RecordingModePrefs.REPEAT_TAP,
                selfHosted = selfHosted,
                selfHostedStyleTitle = selfHostedStyleTitle(appContext),
                customStyles = CustomStylePresets.list(appContext),
                canCreateCustomStyle = CustomStylePresets.canCreate(appContext),
                activeCustomStyleId = CustomStylePresets.activeId(appContext),
                selectedStyle = TextStyle.fromApiId(Prefs.getStylizationStyle(appContext)),
                actionTitle = if (termsRequired) "Принять" else access.actionTitle,
                actionHint = if (termsRequired) "Одно действие" else access.actionHint,
                emojiEnabled = selectedStyleOptions.resolvedEmojiMode.enabled,
                reducedPunctuationEnabled =
                    selectedStyleOptions.punctuation == PunctuationMode.LESS,
            ),
            account = AccountSummaryState(
                available = accountState.available,
                signedIn = accountState.signedIn,
                displayName = accountState.displayName,
                hasOfficialAccess = accountState.hasEntitlement,
                syncing = accountState.syncing,
                syncDecisionPending = accountState.signedIn && accountState.dataSyncMode ==
                    com.govorun.lite.account.AccountDataSyncMode.PENDING,
            ),
            showStarterGuidance = false,
        )
    }

    private fun selfHostedActiveId(context: Context): String? =
        StylizationServerPreferences.selectedStyle(context)
            .takeIf { it.startsWith("custom:") }
            ?.removePrefix("custom:")

    private fun selfHostedStyleTitle(context: Context): String {
        val id = selfHostedActiveId(context) ?: return "Общий"
        return SelfHostedStylePresets.list(context).firstOrNull { it.id == id }?.name ?: "Общий"
    }

    private fun statisticsState(signedIn: Boolean): StatisticsState {
        val cloud = if (signedIn) GovorunAccountRepository.cloudDays() else emptyList()
		val cloudTotal = if (signedIn) GovorunAccountRepository.cloudTotal() else null
        if (cloud.isEmpty() && cloudTotal == null) {
            return StatisticsState(
                wordsToday = StatsStore.getWordsToday(appContext),
                wordsTotal = StatsStore.getWords(appContext),
                voiceSeconds = StatsStore.getSeconds(appContext),
                wordHistory = StatsStore.getWordHistory(appContext, days = 14),
                voiceSecondHistory = StatsStore.getSecondHistory(appContext, days = 14),
            )
        }
        val byDate = cloud.associateBy { it.date }
        val today = LocalDate.now()
        val dates = (13 downTo 0).map { today.minusDays(it.toLong()).toString() }
        return StatisticsState(
            wordsToday = byDate[today.toString()]?.words ?: 0,
			wordsTotal = cloudTotal?.words ?: StatsStore.getWords(appContext),
			voiceSeconds = cloudTotal?.speechSeconds ?: StatsStore.getSeconds(appContext),
            wordHistory = dates.map { byDate[it]?.words ?: 0 },
            voiceSecondHistory = dates.map { byDate[it]?.speechSeconds ?: 0 },
        )
    }
}
