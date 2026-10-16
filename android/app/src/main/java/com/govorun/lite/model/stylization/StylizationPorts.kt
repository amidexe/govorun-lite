package com.govorun.lite.model.stylization

data class OfficialStylizationSnapshot(
    val enabled: Boolean,
    val acceptedTermsVersion: Int,
    val currentTermsVersion: Int,
    val accessState: OfficialStylizationAccessState,
    val serviceAvailable: Boolean,
    val starter: Boolean,
)

data class StylizationSelection(
    val enabled: Boolean,
    val style: TextStyle,
    val instruction: String?,
    val options: StyleOptions,
)

enum class StylizationSkipReason {
    STREAMING_MODE, CUSTOM_STYLE_DRAFT, STYLIZATION_DISABLED,
    ACCESS_INACTIVE, SHORT_PHRASE, APP_RULE_DISABLED,
}

interface StylizationPorts {
    fun customDraftActive(): Boolean
    fun selfHostedSelected(): Boolean
    fun officialState(): OfficialStylizationSnapshot
    fun officialConfigured(): Boolean
    fun resolveStyle(packageName: String?): StylizationSelection
    fun hasInternet(): Boolean
    suspend fun request(text: String, mode: StylizationInputMode,
                        selection: StylizationSelection, characters: Int): StylizationResponse
    suspend fun processSelfHosted(text: String, mode: StylizationInputMode): StylizationResult
    fun didStyle(starter: Boolean)
    fun recordResult(result: StylizationResult)
}

/** Concrete transport/account errors are translated by the platform adapter. */
class StylizationRequestFailure(val reason: StylizationFallbackReason) : Exception()
