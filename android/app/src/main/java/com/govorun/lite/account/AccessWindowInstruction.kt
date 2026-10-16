package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import org.json.JSONObject

internal data class AccessWindowInstruction(val kind: String, val includeTerms: Boolean, val reuseConsent: Boolean)

internal fun accessWindowInstruction(account: AccountState, localTerms: Boolean,
    action: String = "enable", confirmed: Boolean = false, pending: Boolean = false): AccessWindowInstruction {
    val access = account.authority.presentation
    val facts = JSONObject().put("signedIn", account.signedIn)
        .put("storageAvailable", account.secureStorageProblem == null)
        .put("enableAllowed", access.enableAllowed)
        .put("termsActionAvailable", access.termsActionAvailable)
        .put("consentAccepted", access.consentAccepted)
        .put("entitlementActive", account.authority.entitlementActive)
        .put("authorized", account.authority.authorized)
        .put("freeAccessAvailable", account.freeAccessAvailable)
    val result = SharedBusiness.evaluate(JSONObject().put("operation", "accessInteraction")
        .put("facts", facts).put("action", action).put("localTerms", localTerms)
        .put("confirmed", confirmed).put("pending", pending))
    return AccessWindowInstruction(result.getString("kind"), result.getBoolean("includeTerms"),
        result.getBoolean("reuseConsent"))
}
