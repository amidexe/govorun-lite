package com.govorun.lite.stylization

internal enum class StylizationServiceFailure {
    BUSY,
    TIMEOUT,
    UNAVAILABLE,
    INVALID_RESPONSE,
}

internal class StylizationServiceException(
    val failure: StylizationServiceFailure,
    val creditRecoveryHandled: Boolean = false,
) : Exception("Stylization service failure: $failure")
