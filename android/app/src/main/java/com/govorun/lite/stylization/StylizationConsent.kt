package com.govorun.lite.stylization

import android.app.Activity
import android.content.Intent
import android.widget.Toast
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import com.govorun.lite.R
import com.govorun.lite.account.GovorunAccountRepository
import com.govorun.lite.account.accessWindowInstruction
import com.govorun.lite.account.styleSetupNeedsSubscription
import com.govorun.lite.model.stylization.PunctuationMode
import com.govorun.lite.model.stylization.StyleOptions
import com.govorun.lite.model.stylization.TextStyle
import com.govorun.lite.settings.AndroidStylizationSettings
import com.govorun.lite.ui.SubscriptionActivity
import com.govorun.lite.ui.openOnlineProcessingDocuments
import com.govorun.lite.util.Prefs
import com.govorun.lite.util.RecordingModePrefs
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch

object StylizationConsent {

    fun requestEnable(
        activity: Activity,
        onOpenTerms: (() -> Unit)? = null,
        allowBeforeEntitlement: Boolean = false,
        requestedStyle: String? = null,
        acceptAccessAndTerms: (suspend (String, Boolean) -> Boolean)? = null,
        afterSignIn: Boolean = false,
        onResult: (Boolean) -> Unit,
    ) {
        val account = GovorunAccountRepository.state.value
        val localAccepted = Prefs.hasStylizationConsent(activity, StylizationTerms.CURRENT_VERSION)
        val action = if (allowBeforeEntitlement || acceptAccessAndTerms != null) "freeAccess" else "enable"
        val instruction = accessWindowInstruction(account, localAccepted, action)
        if (instruction.kind in listOf("subscription", "signIn") ||
            (!allowBeforeEntitlement && account.authority.entitlementActive && styleSetupNeedsSubscription(account))) {
            activity.startActivity(Intent(activity, SubscriptionActivity::class.java))
            onResult(false)
            return
        }
        if (instruction.kind == "unavailable") {
            Toast.makeText(activity, R.string.stylization_service_temporarily_unavailable, Toast.LENGTH_SHORT).show()
            onResult(false)
            return
        }
        val presets = CustomStylePresets.list(activity)
        val firstActivation = Prefs.getStylizationConsentVersion(activity) == 0
        val choices = TextStyle.entries.filter { it != TextStyle.CUSTOM }.map {
            ConsentStyle(it.apiId, activity.getString(it.titleRes), initialStyleOptions(StylizationStylePreferences.options(activity, it.apiId), firstActivation))
        } + presets.map { ConsentStyle("custom:${it.id}", it.name, initialStyleOptions(StylizationStylePreferences.options(activity, "custom:${it.id}"), firstActivation)) }
        val selected = com.govorun.lite.shared.SharedBusiness.evaluate(org.json.JSONObject().put("operation", "styleLibrary")
            .put("command", org.json.JSONObject().put("kind", "activationSelection").put("firstActivation", firstActivation)
                .put("requested", requestedStyle).put("selected", StylizationStylePreferences.selectedValue(activity))
                .put("available", org.json.JSONArray(choices.map { it.id })))).getString("selected")
        suspend fun applyInstruction(selection: String, options: StyleOptions, confirmed: Boolean): Boolean {
            if (choices.none { it.id == selection }) return false
            val apiStyle = if (selection.startsWith("custom:")) "custom" else selection
            return performStyleAccessInstruction(account, localAccepted,
                currentAccount = { GovorunAccountRepository.state.value },
                activateAccess = { terms -> acceptAccessAndTerms?.invoke(apiStyle, terms)
                    ?: GovorunAccountRepository.activateStarterGrant(activity, terms, apiStyle, account.promoProgram) },
                acceptTerms = { GovorunAccountRepository.acceptOnlineTerms(activity) },
                enableStyle = { recordConsent -> applyStyleSelection(activity, selection, options, recordConsent) },
                action = action, confirmed = confirmed)
        }
        if (instruction.kind != "confirmTerms") {
            val owner = activity as? LifecycleOwner ?: return onResult(false)
            owner.lifecycleScope.launch {
                val enabled = runCatching { applyInstruction(selected,
                    choices.first { it.id == selected }.options, confirmed = false) }.getOrDefault(false)
                currentCoroutineContext().ensureActive()
                if (!enabled) Toast.makeText(activity,
                    "Не удалось включить улучшение. Попробуйте ещё раз.", Toast.LENGTH_SHORT).show()
                onResult(enabled)
            }
            return
        }
        val changesInputMode = RecordingModePrefs.read(activity) != RecordingModePrefs.REPEAT_TAP
        showStylizationConsentScreen(
            activity = activity,
            styles = choices,
            selectedStyle = selected,
            changesInputMode = changesInputMode,
            afterSignIn = afterSignIn,
            onOpenTerms = {
                if (onOpenTerms != null) onOpenTerms()
                else openOnlineProcessingDocuments(activity)
            },
            accept = { selection, options -> applyInstruction(selection, options, confirmed = true) },
            onResult = { accepted ->
                if (!accepted) AndroidStylizationSettings.create(activity).dismissTermsUpdate()
                onResult(accepted)
            },
        )
    }
}

internal fun applyStyleSelection(
    activity: Activity, selection: String, options: StyleOptions, recordConsent: Boolean,
): Boolean {
    if (selection.startsWith("custom:")) {
        if (!AndroidCustomStyles.create(activity).select(selection.removePrefix("custom:"))) return false
    } else {
        Prefs.setStylizationStyle(activity, selection)
        GovorunAccountRepository.recordPortablePreferenceChange(activity, defaultStyle = selection)
    }
    StylizationStylePreferences.setSelectedOptions(activity, options)
    GovorunAccountRepository.recordPortablePreferenceChange(activity,
        emojiEnabled = options.resolvedEmojiMode.enabled, emojiMode = options.resolvedEmojiMode.apiId,
        reducedPunctuation = options.punctuation == PunctuationMode.LESS)
    if (recordConsent) AndroidStylizationSettings.create(activity).acceptTerms(
        StylizationTerms.CURRENT_VERSION, System.currentTimeMillis(),
    )
    RecordingModePrefs.enableStylizationMode(activity)
    AndroidStylizationSettings.create(activity).setEnabled(true)
    GovorunAccountRepository.prepareOfficialStylePassesForUpcomingDictation()
    return true
}
