package com.govorun.lite.ui

import android.content.Intent
import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.govorun.lite.BuildConfig
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.currentOnlineTermsActionRequired
import com.govorun.lite.account.officialStyleSelected
import com.govorun.lite.account.officialStylizationAccessState
import com.govorun.lite.stylization.CustomStylePolicy
import com.govorun.lite.model.stylization.EmojiMode
import com.govorun.lite.model.stylization.OfficialStylizationAccessState
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.settings.AndroidStylizationSettings
import com.govorun.lite.stylization.AndroidCustomStyles
import com.govorun.lite.stylization.CustomStyleDraftSession
import com.govorun.lite.stylization.CustomStylePresets
import com.govorun.lite.stylization.SelfHostedStylePresets
import com.govorun.lite.stylization.StylizationConsent
import com.govorun.lite.stylization.StylizationServerMode
import com.govorun.lite.stylization.StylizationServerPreferences
import com.govorun.lite.stylization.StylizationStylePreferences
import com.govorun.lite.stylization.StylizationTerms
import com.govorun.lite.stylization.styleProcessingPresentation
import com.govorun.lite.ui.design.GovorunTheme
import com.govorun.lite.ui.stylization.StylizationScreen
import com.govorun.lite.ui.stylization.StylizationUiState
import com.govorun.lite.util.Prefs
import kotlinx.coroutines.launch

class StylizationActivity : AppCompatActivity() {
    private val officialConfigured = BuildConfig.OFFICIAL_STYLES_API_URL.startsWith("https://") &&
        BuildConfig.ACCOUNT_API_URL.startsWith("https://")
    private val configured = officialConfigured
    private var state by mutableStateOf(StylizationUiState())
    private var customStyleEditorOpen = false

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        refreshPortablePreferences()
        applyOfficialAccess()
        setContent {
            GovorunTheme {
                StylizationScreen(
                    state = state,
                    openCustomEditorInitially = intent.getBooleanExtra(
                        EXTRA_OPEN_CUSTOM_EDITOR,
                        false,
                    ),
                    onBack = ::finish,
                    onToggle = ::toggleStylization,
                    onEmojiModeChange = ::setEmojiMode,
                    onReducedPunctuationChange = ::setReducedPunctuationEnabled,
                    onProfanityFilterChange = ::setProfanityFilterEnabled,
                    onStyle = ::selectStyle,
                    onSelfHostedGeneral = ::selectSelfHostedGeneral,
                    onCustomStyle = ::selectCustomStyle,
                    onSaveCustomStyle = ::saveCustomStyle,
                    onDeleteCustomStyle = ::deleteCustomStyle,
                    onCustomEditorVisibilityChange = ::setCustomEditorVisible,
                    onReadTerms = { openOnlineProcessingDocuments(this) },
                    onOpenAccess = {
                        startActivity(Intent(this, SubscriptionActivity::class.java))
                    },
                    onAcceptTerms = ::acceptTerms,
                    onAppRules = {
                        startActivity(Intent(this, StylizationAppRulesActivity::class.java))
                    },
                    onOpenSelfHostedServer = {
                        startActivity(Intent(this, SelfHostedServerActivity::class.java))
                    },
                    onDismissNotice = {
                        state = state.copy(notice = null)
                    },
                )
            }
        }
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                GovorunAccountRepository.state.collect {
                    refreshPortablePreferences()
                    applyOfficialAccess()
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        CustomStyleDraftSession.setActive(customStyleEditorOpen)
        refreshPortablePreferences()
        applyOfficialAccess()
    }

    override fun onPause() {
        CustomStyleDraftSession.setActive(false)
        super.onPause()
    }

    private fun setCustomEditorVisible(visible: Boolean) {
        customStyleEditorOpen = visible
        CustomStyleDraftSession.setActive(
            visible && lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED),
        )
    }

    private fun refreshPortablePreferences() {
        state = state.copy(
            punctuationMode = StylizationStylePreferences.optionsForSelected(this).punctuation,
            emojiMode = StylizationStylePreferences.optionsForSelected(this).resolvedEmojiMode,
            profanityFilterEnabled = StylizationStylePreferences.optionsForSelected(this).profanityFilterEnabled,
            selectedStyle = TextStyle.fromApiId(Prefs.getStylizationStyle(this)),
            customInstruction = Prefs.getStylizationCustomInstruction(this),
            customStyles = CustomStylePresets.list(this),
            canCreateCustomStyle = CustomStylePresets.canCreate(this),
            activeCustomStyleId = CustomStylePresets.activeId(this),
        )
    }

    private fun toggleStylization(requested: Boolean) {
        if (state.selfHosted) return
        if (!requested) {
            if (state.selfHosted) {
                StylizationServerPreferences.setStylizationEnabled(this, false)
            } else {
                AndroidStylizationSettings.create(this).setEnabled(false)
            }
            state = state.copy(enabled = false)
            return
        }
        if (!state.canEnable) {
            if (state.selfHosted) {
                startActivity(Intent(this, SelfHostedServerActivity::class.java))
                return
            }
            if (officialConfigured) {
                startActivity(Intent(this, SubscriptionActivity::class.java))
                return
            }
            state = state.copy(
                notice = if (configured) {
                    state.status.ifBlank {
                        getString(R.string.stylization_enable_temporarily_unavailable)
                    }
                } else {
                    getString(R.string.stylization_enable_not_configured)
                },
            )
            return
        }
        if (state.selfHosted) {
            StylizationServerPreferences.setStylizationEnabled(this, true)
            state = state.copy(enabled = true, status = selfHostedStatus())
            return
        }
        StylizationConsent.requestEnable(this) { confirmed ->
            state = state.copy(enabled = confirmed)
        }
    }

    private fun selfHostedStatus(): String = when {
        StylizationServerPreferences.isVerified(this) -> "Сервер работает"
        !StylizationServerPreferences.settings(this).isComplete -> "Настройте свой сервер"
        StylizationServerPreferences.stylizationEnabled(this) -> "Свой сервер · стилизация включена"
        else -> "Свой сервер настроен · стилизация отключена"
    }

    private fun selfHostedStyleTitle(): String {
        val selected = StylizationServerPreferences.selectedStyle(this)
        if (selected == StylizationServerPreferences.GENERAL_STYLE) return "Общий"
        val id = selected.removePrefix("custom:")
        return SelfHostedStylePresets.list(this).firstOrNull { it.id == id }?.name ?: "Общий"
    }

    private fun selfHostedActiveCustomStyleId(): String? =
        StylizationServerPreferences.selectedStyle(this)
            .takeIf { it.startsWith("custom:") }
            ?.removePrefix("custom:")

    private fun applyOfficialAccess() {
        val account = GovorunAccountRepository.state.value
        val localConsent = Prefs.hasStylizationConsent(this, StylizationTerms.CURRENT_VERSION)
        val enabled = officialStyleSelected(Prefs.isStylizationEnabled(this), account)
        val processing = account.styleProcessingPresentation()
        state = state.copy(
            configured = officialConfigured,
            enabled = enabled,
            canEnable = account.hasEntitlement,
            selfHosted = StylizationServerPreferences.mode(this) == StylizationServerMode.SELF_HOSTED,
            serverVerified = false,
            status = officialStatus(),
            hasAccess = account.hasEntitlement,
            termsAccepted = !currentOnlineTermsActionRequired(localConsent, account),
            processingReady = processing.ready,
            processingMessage = processing.message,
        )
    }

    private fun acceptTerms() {
        StylizationConsent.requestEnable(this) { confirmed ->
            val account = GovorunAccountRepository.state.value
            val localConsent = Prefs.hasStylizationConsent(this, StylizationTerms.CURRENT_VERSION)
            state = state.copy(
                enabled = confirmed,
                termsAccepted = !currentOnlineTermsActionRequired(localConsent, account),
            )
        }
    }

    private fun officialStatus(): String {
        val account = GovorunAccountRepository.state.value
        if (Prefs.isStylizationEnabled(this) && currentOnlineTermsActionRequired(
                Prefs.hasStylizationConsent(this, StylizationTerms.CURRENT_VERSION),
                account,
            )
        ) {
            return "Примите обновлённые условия, чтобы продолжить стилизацию"
        }
        return when (officialStylizationAccessState(account)) {
            OfficialStylizationAccessState.SIGN_IN_REQUIRED ->
                "Войдите в аккаунт, чтобы включить улучшение текста"
            OfficialStylizationAccessState.ENTITLEMENT_REQUIRED ->
                "Для стилей нужен активный доступ"
            OfficialStylizationAccessState.TERMS_VERSION_MISMATCH ->
                getString(R.string.stylization_service_temporarily_unavailable)
            OfficialStylizationAccessState.ONLINE_CONSENT_REQUIRED ->
                "Примите условия интернет-обработки"
            OfficialStylizationAccessState.PROTOCOL_UNAVAILABLE ->
                getString(R.string.stylization_service_temporarily_unavailable)
            OfficialStylizationAccessState.READY ->
                account.styleProcessingPresentation().message ?: "Официальные стили доступны"
        }
    }

    private fun setProfanityFilterEnabled(enabled: Boolean) {
        if (state.selfHosted) return
        if (!state.enabled) return
        StylizationStylePreferences.setSelectedOptions(
            this,
            StylizationStylePreferences.optionsForSelected(this).copy(
                profanityFilterEnabled = enabled,
            ),
        )
        GovorunAccountRepository.recordPortablePreferenceChange(
            this,
            profanityFilterEnabled = enabled,
        )
        state = state.copy(profanityFilterEnabled = enabled)
    }

    private fun setEmojiMode(mode: EmojiMode) {
        if (state.selfHosted) return
        if (!state.enabled) return
        StylizationStylePreferences.setSelectedOptions(
            this,
            StylizationStylePreferences.optionsForSelected(this).withEmojiMode(mode),
        )
        GovorunAccountRepository.recordPortablePreferenceChange(
            this,
            emojiEnabled = mode.enabled,
            emojiMode = mode.apiId,
        )
        state = state.copy(emojiMode = mode)
    }

    private fun setReducedPunctuationEnabled(enabled: Boolean) {
        if (state.selfHosted) return
        if (!state.enabled) return
        val punctuation = if (enabled) PunctuationMode.LESS else PunctuationMode.NORMAL
        StylizationStylePreferences.setSelectedOptions(
            this,
            StylizationStylePreferences.optionsForSelected(this).copy(
                punctuation = punctuation,
            ),
        )
        GovorunAccountRepository.recordPortablePreferenceChange(
            this,
            reducedPunctuation = enabled,
            punctuationMode = punctuation.apiId,
        )
        state = state.copy(punctuationMode = punctuation)
    }

    private fun selectStyle(style: TextStyle) {
        if (state.selfHosted) return
        if (!state.enabled) {
            if (!state.canEnable) return
            StylizationConsent.requestEnable(this, requestedStyle = style.apiId) {
                applyOfficialAccess(); refreshPortablePreferences()
            }
            return
        }
        if (style == TextStyle.CUSTOM &&
            !CustomStylePolicy.isValid(state.customInstruction)
        ) {
            return
        }
        if (style != TextStyle.CUSTOM) {
            GovorunAccountRepository.recordSyncedContentChange(this)
        }
        Prefs.setStylizationStyle(this, style.apiId)
        if (style != TextStyle.CUSTOM) {
            GovorunAccountRepository.recordPortablePreferenceChange(this, defaultStyle = style.apiId)
        }
        refreshPortablePreferences()
    }

    private fun selectCustomStyle(id: String) {
        if (state.selfHosted) return
        if (state.selfHosted) {
            if (!state.canEnable || !StylizationServerPreferences.selectCustom(this, id)) return
            StylizationServerPreferences.setStylizationEnabled(this, true)
            applyOfficialAccess()
            return
        }
        if (!state.enabled) {
            if (!state.canEnable) return
            StylizationConsent.requestEnable(this, requestedStyle = "custom:$id") {
                applyOfficialAccess(); refreshPortablePreferences()
            }
            return
        }
        if (!AndroidCustomStyles.create(this).select(id)) return
        refreshPortablePreferences()
    }

    private fun selectSelfHostedGeneral() {
        if (!state.selfHosted || !state.canEnable) return
        StylizationServerPreferences.selectGeneral(this)
        StylizationServerPreferences.setStylizationEnabled(this, true)
        applyOfficialAccess()
    }

    private fun saveCustomStyle(id: String?, name: String, instruction: String, options: com.govorun.lite.model.stylization.StyleOptions) {
        if (state.selfHosted) return
        if (state.selfHosted) {
            val saved = SelfHostedStylePresets.save(this, id, name, instruction) ?: return
            StylizationServerPreferences.selectCustom(this, saved.id)
            StylizationServerPreferences.setStylizationEnabled(this, true)
            applyOfficialAccess()
            return
        }
        AndroidCustomStyles.create(this).saveInLibrary(id, name, instruction, options) ?: return
        refreshPortablePreferences()
    }

    private fun deleteCustomStyle(id: String) {
        if (state.selfHosted) return
        if (state.selfHosted) SelfHostedStylePresets.delete(this, id)
        else AndroidCustomStyles.create(this).delete(id)
        if (state.selfHosted &&
            StylizationServerPreferences.selectedStyle(this) == "custom:$id"
        ) {
            StylizationServerPreferences.selectGeneral(this)
            applyOfficialAccess()
            return
        }
        refreshPortablePreferences()
    }

    private fun setCustomInstruction(value: String) {
        if (!state.enabled) return
        val instruction = CustomStylePolicy.normalize(value)
        if (!CustomStylePolicy.isValid(instruction)) return
        AndroidCustomStyles.create(this).saveActiveInstruction(instruction) ?: return
        state = state.copy(
            selectedStyle = TextStyle.CUSTOM,
            customInstruction = instruction,
        )
    }

    companion object {
        const val EXTRA_OPEN_CUSTOM_EDITOR = "open_custom_style_editor"
    }
}
