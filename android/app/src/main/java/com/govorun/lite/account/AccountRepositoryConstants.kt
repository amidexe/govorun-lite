package com.govorun.lite.account

import android.content.Context
import com.govorun.lite.stylization.StylizationTerms
import com.govorun.lite.stylization.StylizationServerPreferences
import com.govorun.lite.stylization.StylizationServerMode
import com.govorun.lite.util.Prefs


internal fun canIssueStylePasses(context: Context, state: AccountState): Boolean =
    styleScopeDecision(org.json.JSONObject().put("kind", "enabled")
        .put("selfHosted", StylizationServerPreferences.mode(context) == StylizationServerMode.SELF_HOSTED)
        .put("enabled", Prefs.isStylizationEnabled(context)).put("processingReady", state.processingReady)
        .put("protocolSupported", state.styleAccess.supportsTextCreditProtocol)
        .put("consent", Prefs.hasStylizationConsent(context, StylizationTerms.CURRENT_VERSION))).getBoolean("value")
