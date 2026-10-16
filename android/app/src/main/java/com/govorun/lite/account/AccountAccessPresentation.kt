package com.govorun.lite.account

import com.govorun.lite.model.stylization.OfficialStylizationAccessState
import org.json.JSONObject

/** Only decodes the common session's display decisions. */
data class AccountAccessPresentation(
    val state: OfficialStylizationAccessState = OfficialStylizationAccessState.SIGN_IN_REQUIRED,
    val enableAllowed: Boolean = false,
    val termsActionAvailable: Boolean = false,
    val consentAccepted: Boolean = false,
    val displayEntitlementActive: Boolean = false,
) {
    companion object {
        internal fun read(value: JSONObject) = AccountAccessPresentation(
            state = when (value.getString("state")) {
                "signInRequired" -> OfficialStylizationAccessState.SIGN_IN_REQUIRED
                "entitlementRequired" -> OfficialStylizationAccessState.ENTITLEMENT_REQUIRED
                "termsVersionMismatch" -> OfficialStylizationAccessState.TERMS_VERSION_MISMATCH
                "onlineConsentRequired" -> OfficialStylizationAccessState.ONLINE_CONSENT_REQUIRED
                "protocolUnavailable" -> OfficialStylizationAccessState.PROTOCOL_UNAVAILABLE
                "ready" -> OfficialStylizationAccessState.READY
                else -> error("Unknown common access presentation")
            }, enableAllowed = value.getBoolean("enableAllowed"),
            termsActionAvailable = value.getBoolean("termsActionAvailable"),
            consentAccepted = value.getBoolean("consentAccepted"),
            displayEntitlementActive = value.getBoolean("displayEntitlementActive"),
        )
    }
}
