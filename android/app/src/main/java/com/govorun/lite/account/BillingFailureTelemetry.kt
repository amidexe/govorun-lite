package com.govorun.lite.account

import org.json.JSONObject

internal fun tBankCheckoutFailureClass(error: Throwable): String {
    if (error !is AccountHttpException) return error.javaClass.simpleName.ifBlank { "unknown" }
    val code = runCatching {
        JSONObject(error.responseBody).optString("error").takeIf(::validBillingErrorCode)
    }.getOrNull()
    return "http_${error.statusCode}" + (code?.let { "_$it" } ?: "")
}

private fun validBillingErrorCode(value: String): Boolean =
    value.length in 1..64 && value.all { it.isLowerCase() || it.isDigit() || it == '_' }
