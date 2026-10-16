package com.govorun.lite.account

import com.govorun.lite.shared.SharedBusiness
import com.govorun.lite.shared.SharedBusinessException
import org.json.JSONObject
import org.json.JSONArray
import java.io.IOException

internal class PromoCodeInputException : IllegalArgumentException("Invalid Promo code")

internal fun normalizePromoCode(value: String): String = try {
    SharedBusiness.evaluate(JSONObject().put("operation", "normalizePromotion").put("code", value))
        .getString("code")
} catch (error: SharedBusinessException) {
    if (error.status == 400) throw PromoCodeInputException()
    throw error
}

internal object SharedPromotionRules {
    fun message(error: Throwable): String {
        val status = when (error) {
            is AccountHttpException -> error.statusCode
            is SharedBusinessException -> error.status.takeIf { it != 0 }
            is PromoCodeInputException -> 400
            else -> null
        }
        val code = when (error) {
            is AccountHttpException -> runCatching { JSONObject(error.responseBody).optString("error") }.getOrNull()
            is SharedBusinessException -> error.code
            is IOException -> "account_network_unavailable"
            else -> null
        }
        return try {
            SharedBusiness.evaluate(JSONObject().put("operation", "promotionFailure")
                .put("status", status ?: JSONObject.NULL).put("error", code ?: JSONObject.NULL)).getString("message")
        } catch (_: SharedBusinessException) {
            "Проверка промокодов недоступна. Попробуйте перезапустить приложение"
        }
    }

    fun requiresCatalog(offer: JSONObject): Boolean = SharedBusiness.evaluate(
        JSONObject().put("operation", "inspectPromotionOffer").put("offer", offer),
    ).getBoolean("requiresCatalog")

    fun validate(offer: JSONObject, catalog: JSONObject?, requestedOffer: String?) {
        SharedBusiness.evaluate(JSONObject().put("operation", "validatePromotionOffer")
            .put("offer", offer).put("catalog", catalog ?: JSONObject.NULL)
            .put("requestedOffer", requestedOffer?.takeIf(String::isNotEmpty) ?: JSONObject.NULL))
    }
}

internal fun promoOfferHint(value: String, plans: List<BillingPlan>): String =
    SharedBusiness.evaluate(JSONObject().put("operation", "resolvePromotionOfferHint")
        .put("requestedOffer", value).put("availableOffers", JSONArray(plans.map { it.offerId })))
        .getString("offerId")
