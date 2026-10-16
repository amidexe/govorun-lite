package com.govorun.lite.account


data class GovorunSession(
    val accessToken: String,
    val refreshToken: String,
    val displayName: String,
    val accountId: String? = null,
    val onlineTermsVersion: Int = 0,
    val deviceId: String = "",
    val deviceLimit: Int = 5,
    val styleAccess: StyleAccess = StyleAccess(),
    val starterGrant: StarterGrant = StarterGrant(),
    val promoProgram: PromoProgram = PromoProgram(),
    val freeAccessAvailable: Boolean = false,
    val onlineConsent: OnlineConsent = OnlineConsent(),
    val onlineService: OnlineService = OnlineService(),
    val styleUsageAccountingAvailable: Boolean = false,
    // Fresh profile metadata only; never persist an "empty" assumption across sessions.
    val hasSyncedData: Boolean? = null,
)

data class OnlineService(
    val state: OnlineServiceState = OnlineServiceState.UNKNOWN,
) {
    val available: Boolean get() = state == OnlineServiceState.AVAILABLE
}

enum class OnlineServiceState(val wireValue: String) {
    AVAILABLE("available"),
    CLOSED("closed"),
    UNKNOWN("unknown");

    companion object {
        fun fromWire(value: String?): OnlineServiceState = entries.firstOrNull {
            it != UNKNOWN && it.wireValue == value
        } ?: UNKNOWN
    }
}

typealias StyleAccess = com.govorun.core.generated.ConsumerStyleAccess

typealias StarterGrant = com.govorun.lite.model.billing.StarterGrant

typealias OnlineConsent = com.govorun.core.generated.ConsumerOnlineConsent

typealias BillingPlan = com.govorun.lite.model.billing.BillingPlan
typealias BillingCatalog = com.govorun.lite.model.billing.BillingCatalog
typealias StarterGrantOffer = com.govorun.lite.model.billing.StarterGrantOffer
typealias BillingTrialOffer = com.govorun.lite.model.billing.BillingTrialOffer
typealias BillingStatus = com.govorun.lite.model.billing.BillingStatus
typealias BillingSubscription = com.govorun.lite.model.billing.BillingSubscription
typealias BillingRenewalIssue = com.govorun.lite.model.billing.BillingRenewalIssue
typealias BillingNextCharge = com.govorun.lite.model.billing.BillingNextCharge
typealias BillingRefundRequest = com.govorun.lite.model.billing.BillingRefundRequest
typealias BillingPlanChange = com.govorun.lite.model.billing.BillingPlanChange
typealias TBankCheckoutPreview = com.govorun.lite.model.billing.TBankCheckoutPreview
typealias TBankLegalDocument = com.govorun.lite.model.billing.TBankLegalDocument
typealias TBankLegalDocuments = com.govorun.lite.model.billing.TBankLegalDocuments
typealias TBankCheckoutSession = com.govorun.lite.model.billing.TBankCheckoutSession
typealias BillingUiState = com.govorun.lite.model.billing.BillingUiState

typealias AccountDevice = com.govorun.core.generated.ConsumerAccountDevice

data class AccountAuthority(
    val entitlementActive: Boolean = false,
    val authorized: Boolean = false,
    val processingReady: Boolean = false,
    val presentation: AccountAccessPresentation = AccountAccessPresentation(),
)

data class AccountState(
    val authority: AccountAuthority = AccountAuthority(),
    val available: Boolean = false,
    val networkAvailable: Boolean = true,
    val profileFresh: Boolean = true,
    val signedIn: Boolean = false,
    val accountId: String = "",
    val displayName: String = "",
    val onlineTermsVersion: Int = 0,
    val syncing: Boolean = false,
    val deletingAccount: Boolean = false,
    val secureStorageProblem: SecureStorageProblem? = null,
    val message: String? = null,
    val devices: List<AccountDevice> = emptyList(),
    val deviceLimit: Int = 5,
    val deviceLimitReached: Boolean = false,
    val dataSyncMode: AccountDataSyncMode = AccountDataSyncMode.EVALUATING,
    val styleAccess: StyleAccess = StyleAccess(),
    val starterGrant: StarterGrant = StarterGrant(),
    val promoProgram: PromoProgram = PromoProgram(),
    val freeAccessAvailable: Boolean = false,
    val onlineConsent: OnlineConsent = OnlineConsent(),
    val onlineService: OnlineService = OnlineService(),
    val pendingStyleUsageToday: Int = 0,
    val pendingStyleUsageTotal: Int = 0,
    val localUnspentStyleCredits: Int = 0,
    val localPendingStyleCredits: Int = 0,
    val localStylePassIssuancePending: Boolean = false,
    val localStylePassStateKnown: Boolean = false,
    val styleWindowJson: String? = null,
    val profileGeneration: Long = 0,
    val sessionGeneration: Long = 0,
) {
    val starterLocallyExhausted: Boolean
        get() = styleWindowPresentation().starterLocallyExhausted

    /** These values are projected by the shared owner, never inferred from a cache. */
    val hasEntitlement: Boolean get() = authority.entitlementActive || authority.presentation.displayEntitlementActive
    val hasOfficialAccess: Boolean get() = styleControls().getBoolean("authorized")
    val processingReady: Boolean
        get() = styleControls().getBoolean("processingReady")

}

enum class AuthProvider(val apiName: String, val callbackPath: String, val title: String) {
    YANDEX("yandex", "/yandex", "Яндекс"),
    VK("vk", "/vk", "VK ID"),
}

internal data class PortablePreferenceChanges(
    val defaultStyle: String? = null,
    val reducedPunctuation: Boolean? = null,
    val punctuationMode: String? = null,
    val emojiEnabled: Boolean? = null,
    val emojiMode: String? = null,
    val profanityFilterEnabled: Boolean? = null,
    val shortPhraseThreshold: Int? = null,
) {
    val isEmpty: Boolean
        get() = defaultStyle == null && reducedPunctuation == null && punctuationMode == null &&
            emojiEnabled == null && emojiMode == null && profanityFilterEnabled == null &&
            shortPhraseThreshold == null
}

internal data class SyncedDictionary(
    val enabled: Boolean,
    val text: String,
)

internal data class SyncedCustomStyle(
    val id: String,
    val name: String,
    val instruction: String,
    val punctuationMode: String,
    val emojiEnabled: Boolean,
    val emojiMode: String = if (emojiEnabled) "bright" else "off",
    val profanityFilterEnabled: Boolean,
)

internal data class SyncedContentDocument(
    val schemaVersion: Int = 1,
    val dictionary: SyncedDictionary = SyncedDictionary(false, ""),
    val selectedCustomStyleId: String = "",
    val customStyles: List<SyncedCustomStyle> = emptyList(),
)

internal data class SyncDocumentResponse(
    val configured: Boolean,
    val etag: String,
    val document: SyncedContentDocument,
    val notModified: Boolean = false,
)

internal data class AccountHttpResponse(
    val statusCode: Int,
    val body: String,
    val etag: String? = null,
    val retryAfterMillis: Long? = null,
)

internal class AccountHttpException(
    val statusCode: Int,
    val responseBody: String = "",
    val retryAfterMillis: Long? = null,
    val checkoutFailure: CheckoutConfirmationFailure? = null,
) :
    Exception("Account HTTP $statusCode")

internal data class CheckoutConfirmationFailure(val discardConfirmation: Boolean, val message: String)

internal class CheckoutConfirmationException(val failure: CheckoutConfirmationFailure) :
    IllegalStateException(failure.message)

internal fun checkoutConfirmationFailure(error: Throwable): CheckoutConfirmationFailure? = when (error) {
    is CheckoutConfirmationException -> error.failure
    is AccountHttpException -> error.checkoutFailure
    else -> null
}

internal class AccountDeviceLimitException(
    val deviceLimit: Int,
    val devices: List<AccountDevice>,
) : Exception("Account device limit reached")
