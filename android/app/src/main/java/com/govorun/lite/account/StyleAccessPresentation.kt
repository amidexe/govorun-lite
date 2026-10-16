package com.govorun.lite.account

internal val StyleAccess.active: Boolean get() = state == "active"
internal val StyleAccess.supportsReserveReturn: Boolean get() = sharedFacts().getBoolean("reserveReturn")
internal val StyleAccess.supportsTextCreditProtocol: Boolean get() = sharedFacts().getBoolean("protocolSupported")
private fun StyleAccess.sharedFacts() = styleScopeDecision(org.json.JSONObject().put("kind", "facts").put("access", org.json.JSONObject()
    .put("quotaUnit", quotaUnit).put("charactersPerCredit", charactersPerCredit).put("maximumCreditsPerRequest", maximumCreditsPerRequest)
    .put("stylizationProtocol", stylizationProtocol).put("reserveReturnProtocol", reserveReturnProtocol)))
internal val StarterGrant.allocated: Boolean get() = state == "allocated"
