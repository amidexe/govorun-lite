package com.govorun.lite.model.stylization

data class StylizationResult(
    val text: String,
    val usedFallback: Boolean = false,
    val outcomePass: String? = null,
    val latencyBucket: String = "unknown",
    val fallbackReason: StylizationFallbackReason? = null,
    val skipReason: StylizationSkipReason? = null,
)

enum class StylizationFallbackReason {
    TERMS_UPDATE_REQUIRED,
    SERVICE_UNAVAILABLE,
    TEXT_TOO_LONG,
    NOT_CONFIGURED,
    OFFLINE,
    NO_CHANGE,
    STYLE_PASS_RESERVE_EMPTY,
    STYLE_PASS_ISSUE_FAILED,
    STYLE_PASS_RECOVERY_PENDING,
    ACCOUNT_STATE_CHANGED,
    TRANSPORT_FAILED,
    REQUEST_FAILED,
    SERVER_BUSY,
    SERVER_TIMEOUT,
    SERVER_UNAVAILABLE,
    INVALID_RESPONSE,
}
