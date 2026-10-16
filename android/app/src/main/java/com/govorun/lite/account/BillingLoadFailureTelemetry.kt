package com.govorun.lite.account

internal fun billingLoadFailureClass(error: Throwable): String = when (error) {
    is AccountHttpException -> "http_${error.statusCode}"
    is IllegalArgumentException -> "catalog_contract"
    is IllegalStateException -> "session_or_build_state"
    else -> error::class.simpleName?.lowercase() ?: "unknown"
}
