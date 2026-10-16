package com.govorun.lite.account

import org.json.JSONObject

/** Commands are native DTOs; admission, sequencing and recovery belong to Rust. */
sealed interface SubscriptionCommand {
    data class PreviewPromotion(val code: String, val offerId: String) : SubscriptionCommand
    data class ConfirmPromotion(val id: String) : SubscriptionCommand
    data object DismissPromotion : SubscriptionCommand
    data class RefreshPromotionAccess(val id: String) : SubscriptionCommand
    data class DismissPromotionAccess(val id: String, val deferred: Boolean) : SubscriptionCommand
    data object Refresh : SubscriptionCommand
    data object Recover : SubscriptionCommand
    data object RefreshByUser : SubscriptionCommand
    data object RecoverByUser : SubscriptionCommand
    data object Dismiss : SubscriptionCommand
    data object Resume : SubscriptionCommand
    data object RestartBinding : SubscriptionCommand
    data class Preview(val offerId: String, val promoId: String = "") : SubscriptionCommand
    data class ChooseMethod(val previewId: String, val method: String) : SubscriptionCommand
    data class Confirm(val previewId: String, val email: String?) : SubscriptionCommand
    data class Manage(val action: String, val reason: String? = null) : SubscriptionCommand
}

sealed interface SubscriptionEffect {
    data class OpenUrl(val url: String) : SubscriptionEffect
    data class PromotionAccess(val value: com.govorun.lite.account.PromotionAccess, val continuationId: String?) : SubscriptionEffect
}

data class SubscriptionWindow(
    val busy: Boolean = false,
    val previewId: String? = null,
    val preview: TBankCheckoutPreview? = null,
    val message: String? = null,
    val failed: Boolean = false,
    val canResume: Boolean = false,
    val canCancel: Boolean = false,
    val checkoutUrl: String? = null,
    val expiresAt: Long? = null,
    val disposition: String? = null,
    val checkoutUnavailable: Boolean = false,
)

internal fun SubscriptionCommand.wire(catalog: BillingCatalog): JSONObject = JSONObject().apply {
    put("client", JSONObject().put("channel", "direct")
        .put("bankPresentation", "browser").put("preferredPaymentMethod", "sbp"))
    when (val command = this@wire) {
        is SubscriptionCommand.PreviewPromotion -> put("action", "previewPromo")
            .put("code", command.code).put("offerId", command.offerId)
        is SubscriptionCommand.ConfirmPromotion -> put("action", "confirmPromo")
            .put("previewId", command.id).put("confirmed", true).put("displayedCatalog", catalog.sharedWire())
        SubscriptionCommand.DismissPromotion -> put("action", "dismissPromo")
        is SubscriptionCommand.RefreshPromotionAccess -> put("action", "refreshPromoAccess").put("continuationId", command.id)
        is SubscriptionCommand.DismissPromotionAccess -> put("action", "dismissPromoAccess")
            .put("continuationId", command.id).put("deferred", command.deferred)
        SubscriptionCommand.Refresh, SubscriptionCommand.RefreshByUser -> put("action", "refresh")
        SubscriptionCommand.Recover, SubscriptionCommand.RecoverByUser -> put("action", "recover")
        SubscriptionCommand.Dismiss -> put("action", "dismissPreview")
        SubscriptionCommand.Resume -> put("action", "resumeCheckout")
        SubscriptionCommand.RestartBinding -> put("action", "restartBinding")
        is SubscriptionCommand.Preview -> {
            put("action", "previewCheckout").put("offerId", command.offerId)
            if (command.promoId.isNotEmpty()) put("promoPreviewId", command.promoId)
            put("displayedCatalog", catalog.sharedWire())
        }
        is SubscriptionCommand.ChooseMethod -> put("action", "choosePaymentMethod")
            .put("previewId", command.previewId).put("method", command.method)
        is SubscriptionCommand.Confirm -> put("action", "confirmCheckout").put("previewId", command.previewId)
            .put("confirmed", true).put("email", command.email ?: JSONObject.NULL)
        is SubscriptionCommand.Manage -> put("action", command.action).put("confirmed", true)
            .put("reasonCode", command.reason ?: JSONObject.NULL)
    }
    put("userInitiated", this@wire != SubscriptionCommand.Refresh && this@wire != SubscriptionCommand.Recover)
}

internal fun JSONObject.subscriptionWindow(epoch: Long): SubscriptionWindow {
    val value = getJSONObject("subscriptionWindow")
    require(value.getLong("epoch") == epoch)
    fun text(key: String) = value.get(key).takeUnless { it == JSONObject.NULL } as String?
    return SubscriptionWindow(value.getBoolean("busy"), text("previewId"),
        value.get("preview").takeUnless { it == JSONObject.NULL }?.let { (it as JSONObject).toTBankCheckoutPreview() },
        text("message"), value.getBoolean("failed"), value.getBoolean("canResume"), value.getBoolean("canCancel"),
        text("checkoutUrl"), text("expiresAt")?.let { java.time.Instant.parse(it).toEpochMilli() },
        value.optJSONObject("outcome")?.getJSONObject("clientCheckout")?.getString("disposition"),
        value.getBoolean("checkoutUnavailable"))
}
