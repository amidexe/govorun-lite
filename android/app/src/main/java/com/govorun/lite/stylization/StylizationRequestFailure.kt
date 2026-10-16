package com.govorun.lite.stylization

import com.govorun.lite.account.AccountSessionChangedException
import com.govorun.lite.model.stylization.StylizationFallbackReason
import com.govorun.lite.network.OfficialSessionRevokedException
import java.io.IOException

internal fun requestFailureReason(error: Exception): StylizationFallbackReason {
    if (error is com.govorun.lite.shared.SharedBusinessException) {
        val decision = error.stylizationFailure ?: throw error
        return StylizationFallbackReason.valueOf(decision.getString("reason").uppercase())
    }
    val family = when (error) {
        is com.govorun.lite.account.SecureStorageUnavailableException, is AccountSessionChangedException, is OfficialSessionRevokedException -> "account"
        is StylizationServiceException -> "service"
        is StylizationHttpException -> "http"
        is IOException -> "transport"
        else -> "unknown"
    }
    return StylizationFallbackReason.valueOf(diagnosticCommand(org.json.JSONObject().put("kind", "requestFailure")
        .put("family", family).put("status", (error as? StylizationHttpException)?.statusCode)
        .put("code", (error as? StylizationServiceException)?.failure?.name)).getString("reason"))
}
