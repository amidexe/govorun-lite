package com.govorun.lite.ui.home

import com.govorun.lite.account.AccountState
import com.govorun.lite.model.stylization.CustomStylePreset
import com.govorun.lite.model.stylization.TextStyle

data class HomeUiState(
    val ready: Boolean,
    val headline: String,
    val supportingText: String,
    val issues: List<SetupIssue>,
    val statistics: StatisticsState,
    val dictionary: DictionarySummaryState = DictionarySummaryState(),
    val stylization: StylizationOfferState,
    val account: AccountSummaryState,
    val showStarterGuidance: Boolean = false,
    val lastDictation: String? = null,
    val lastDictationOriginal: String? = null,
) {
    companion object {
        fun loading() = HomeUiState(
            ready = false,
            headline = "Говорун",
            supportingText = "Проверяем готовность…",
            issues = emptyList(),
            statistics = StatisticsState(),
            stylization = StylizationOfferState(),
            account = AccountSummaryState(),
            showStarterGuidance = false,
        )
    }
}

data class StatisticsState(
    val wordsToday: Long = 0,
    val wordsTotal: Long = 0,
    val voiceSeconds: Long = 0,
    val wordHistory: List<Long> = List(14) { 0L },
    val voiceSecondHistory: List<Long> = List(14) { 0L },
)

data class DictionarySummaryState(
    val enabled: Boolean = false,
    val visible: Boolean = false,
)

internal fun shouldShowDictionaryCard(state: DictionarySummaryState): Boolean = state.visible

internal fun dictionaryStatusLabel(state: DictionarySummaryState): String =
    if (state.enabled) "Включён" else "Выключен"

data class StylizationOfferState(
    val signedIn: Boolean = false,
    val retryConnection: Boolean = false,
    val pauseMessage: String? = null,
    val visible: Boolean = false,
    val status: String = "Проверяем доступность…",
    val accessLabel: String = "Улучшение текста",
    val actionEnabled: Boolean = false,
    val accessActive: Boolean = false,
    val starterAvailable: Boolean = false,
    val enabled: Boolean = false,
    val processingReady: Boolean = false,
    val processingMessage: String? = null,
    val processingFailed: Boolean = false,
    val processingRevision: Long = 0,
    val termsRequired: Boolean = false,
    val canEnableDirectly: Boolean = false,
    val selfHosted: Boolean = false,
    val selfHostedStyleTitle: String = "Общий",
    val customStyles: List<CustomStylePreset> = emptyList(),
    val canCreateCustomStyle: Boolean = false,
    val activeCustomStyleId: String? = null,
    val selectedStyle: TextStyle = TextStyle.FORMAL,
    val actionTitle: String = "Настроить",
    val actionHint: String = "Выберите вариант доступа",
    val emojiEnabled: Boolean = false,
    val reducedPunctuationEnabled: Boolean = false,
) {
    val readyForProcessing: Boolean
        get() = enabled && (selfHosted || processingReady) && pauseMessage == null && !termsRequired

    val accessEvent: HomeEvent get() = when {
        retryConnection && signedIn -> HomeEvent.RetryConnection
        selfHosted -> HomeEvent.OpenStylization
        termsRequired -> HomeEvent.RequestStylizationConsent
        !signedIn -> HomeEvent.StartStylization
        starterAvailable -> HomeEvent.RequestStylizationConsent
        else -> HomeEvent.OpenSubscription
    }
    val styleSetupEvent: HomeEvent
        get() = if (selfHosted) {
            HomeEvent.OpenStylization
        } else if (accessActive) {
            HomeEvent.RequestStylizationConsent
        } else {
            accessEvent
        }

    val homeCard: HomeStylizationCard
        get() = when {
            !visible -> HomeStylizationCard.HIDDEN
            termsRequired -> HomeStylizationCard.ACCESS_SETUP
            accessActive -> HomeStylizationCard.STYLE_PICKER
            else -> HomeStylizationCard.ACCESS_SETUP
        }
}

enum class HomeStylizationCard {
    HIDDEN,
    ACCESS_SETUP,
    STYLE_PICKER,
}

internal fun shouldShowStarterGuidance(account: AccountState, locallyDismissed: Boolean): Boolean =
    !locallyDismissed && account.signedIn && account.styleAccess.type == "starter" &&
        account.hasEntitlement

data class AccountSummaryState(
    val available: Boolean = false,
    val signedIn: Boolean = false,
    val displayName: String = "",
    val hasOfficialAccess: Boolean = false,
    val syncing: Boolean = false,
    val syncDecisionPending: Boolean = false,
)

enum class SetupIssue {
    MICROPHONE,
    ACCESSIBILITY,
    ACCESSIBILITY_RESTART,
    AUTOSTART,
    ACCESSIBILITY_SHORTCUT,
    BATTERY,
}

sealed interface HomeEvent {
    data object OpenSettings : HomeEvent
    data object OpenAbout : HomeEvent
    data object OpenSubscription : HomeEvent
    data object RetryConnection : HomeEvent
    data object OpenStylization : HomeEvent
    data object RequestStylizationConsent : HomeEvent
    data object StartStylization : HomeEvent
    data object OpenDictionary : HomeEvent
    data object DisableStylization : HomeEvent
    data object OpenAccount : HomeEvent
    data object MergeAccountData : HomeEvent
    data object KeepAccountDataLocal : HomeEvent
    data object OpenReview : HomeEvent
    data object OpenProducts : HomeEvent
    data object OpenAccessibility : HomeEvent
    data object OpenAutostart : HomeEvent
    data object OpenAppDetails : HomeEvent
    data object RequestBattery : HomeEvent
    data object DismissStarterGuidance : HomeEvent
    data class DismissStyleFeedback(val revision: Long) : HomeEvent
    data class SelectStyle(val style: TextStyle) : HomeEvent
    data class SelectCustomStyle(val id: String) : HomeEvent
    data object SelectSelfHostedGeneral : HomeEvent
    data class SaveCustomStyle(
        val id: String?,
        val name: String,
        val instruction: String,
    ) : HomeEvent
    data class DeleteCustomStyle(val id: String) : HomeEvent
}

internal fun Long.compactNumber(): String = when {
    this < 1_000 -> toString()
    this < 1_000_000 -> {
        val value = this / 1_000.0
        if (value >= 10 || value % 1.0 == 0.0) "${value.toInt()} тыс."
        else "${String.format(java.util.Locale.forLanguageTag("ru"), "%.1f", value)} тыс."
    }
    else -> {
        val value = this / 1_000_000.0
        "${String.format(java.util.Locale.forLanguageTag("ru"), "%.1f", value)} млн"
    }
}
