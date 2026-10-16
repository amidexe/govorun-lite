package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusinessException
import org.json.JSONObject

data class PromotionPreview(val id: String, val title: String, val body: String,
    val checkoutRequired: Boolean, val bonusDays: Int?)
data class PromotionContinuation(val id: String, val access: PromotionAccess)
data class PromotionWindow(val busy: Boolean = false, val preview: PromotionPreview? = null,
    val message: String? = null, val continuation: PromotionContinuation? = null)

internal fun JSONObject.promotionWindow(epoch: Long): PromotionWindow {
    val value = getJSONObject("promotionWindow")
    require(value.getLong("epoch") == epoch)
    val preview = value.optJSONObject("promo")?.let {
        PromotionPreview(it.getString("id"), it.getString("title"), it.getString("body"),
            it.getBoolean("checkoutRequired"), if (it.isNull("bonusDays")) null else it.getInt("bonusDays"))
    }
    return PromotionWindow(value.getBoolean("busy"), preview,
        value.get("message").takeUnless { it == JSONObject.NULL } as String?,
        value.optJSONObject("promoAccess")?.let { PromotionContinuation(it.getString("id"), it.toPromotionAccess()) })
}

internal fun JSONObject.toPromotionAccess(): PromotionAccess {
    val kind = when (getString("kind")) {
        "done" -> PromotionAccessKind.DONE
        "acceptTerms" -> PromotionAccessKind.ACCEPT_TERMS
        "refresh" -> PromotionAccessKind.REFRESH
        else -> throw SharedBusinessException("account_invalid_response", 0)
    }
    val terms = if (isNull("termsVersion")) null else getInt("termsVersion")
    require((kind == PromotionAccessKind.ACCEPT_TERMS) == (terms != null) && (terms == null || terms > 0))
    return PromotionAccess(kind, terms, getString("message"), getString("deferredMessage"))
}
