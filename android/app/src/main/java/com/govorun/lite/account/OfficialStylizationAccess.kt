package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.model.stylization.OfficialStylizationAccessState
import com.govorun.lite.stylization.StylizationTerms
import com.govorun.lite.util.Prefs

internal enum class StylizationEnableAction { CONTINUE, BLOCK_WITH_PASSIVE_NOTICE }

internal fun stylizationEnableAction(account: AccountState): StylizationEnableAction =
    StylizationEnableAction.valueOf(account.styleControls().getString("enableAction"))

internal fun officialStylizationAccessState(account: AccountState): OfficialStylizationAccessState =
    account.authority.presentation.state

/** The local checkbox changes presentation, never server consent or admission. */
internal fun currentOnlineTermsActionRequired(localCurrentTermsAccepted: Boolean, account: AccountState): Boolean =
    account.styleControls(localCurrentTermsAccepted = localCurrentTermsAccepted).getBoolean("termsRequired")

internal fun AccountState.hasCurrentOnlineConsent(): Boolean = authority.presentation.consentAccepted

internal fun officialStylizationActive(enabledPreference: Boolean, localConsent: Boolean, account: AccountState): Boolean =
    account.styleControls(enabledPreference, localConsent).getBoolean("active")

/** The user's saved style choice is presentation state, not network authorization. */
internal fun officialStyleSelected(enabledPreference: Boolean, account: AccountState): Boolean =
    account.styleControls(enabledPreference).getBoolean("selected")

internal fun officialStylizationActive(context: Context): Boolean = officialStylizationActive(
    Prefs.isStylizationEnabled(context), Prefs.hasStylizationConsent(context, StylizationTerms.CURRENT_VERSION),
    GovorunAccountRepository.state.value,
)

internal fun AccountState.styleControls(enabled: Boolean = false, localConsent: Boolean = false,
    localCurrentTermsAccepted: Boolean = false): org.json.JSONObject = styleScopeDecision(org.json.JSONObject()
        .put("kind", "controls").put("facts", org.json.JSONObject().put("signedIn", signedIn)
            .put("storageAvailable", secureStorageProblem == null).put("entitlementActive", authority.entitlementActive)
            .put("displayEntitlementActive", authority.presentation.displayEntitlementActive)
            .put("authorized", authority.authorized).put("processingReady", authority.processingReady)
            .put("enableAllowed", authority.presentation.enableAllowed).put("termsActionAvailable", authority.presentation.termsActionAvailable)
            .put("consentAccepted", authority.presentation.consentAccepted).put("enabled", enabled).put("localConsent", localConsent)
            .put("localCurrentTermsAccepted", localCurrentTermsAccepted)))

internal fun styleScopeDecision(command: org.json.JSONObject): org.json.JSONObject =
    com.govorun.lite.shared.SharedBusiness.evaluate(org.json.JSONObject()
        .put("operation", "styleScope").put("command", command))
