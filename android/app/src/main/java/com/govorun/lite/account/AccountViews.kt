package com.govorun.lite.account

import com.govorun.core.generated.*
import com.govorun.lite.shared.SharedBusiness
import org.json.JSONArray
import org.json.JSONObject

/** Mechanical projection decoding. Core owns wire defaults and validation. */
internal fun JSONObject.consumerView(kind: String, version: String = ""): JSONObject = SharedBusiness.evaluate(
    JSONObject().put("operation", "consumerView").put("kind", kind).put("value", this).put("version", version))

internal fun JSONObject.toStyleAccess() = ConsumerStyleAccess.fromJson(consumerView("styleAccess"))
internal fun JSONObject.toOnlineConsent() = ConsumerOnlineConsent.fromJson(consumerView("onlineConsent"))
internal fun JSONObject.toStarterGrant() = ConsumerStarterGrant.fromJson(consumerView("starterGrant"))
internal fun JSONObject.toStarterGrantOffer() = ConsumerStarterGrantOffer.fromJson(consumerView("starterGrantOffer"))
internal fun JSONObject.toBillingTrialOffer() = ConsumerBillingTrialOffer.fromJson(consumerView("billingTrialOffer"))
internal fun JSONObject.toBillingStatus() = ConsumerBillingStatus.fromJson(consumerView("billingStatus"))
internal fun JSONObject.toBillingPlan() = ConsumerBillingPlan.fromJson(consumerView("billingPlan"))
internal fun JSONObject.toBillingSubscription() = ConsumerBillingSubscription.fromJson(consumerView("billingSubscription"))
internal fun JSONObject.toBillingRenewalIssue() = ConsumerBillingRenewalIssue.fromJson(consumerView("billingRenewalIssue"))
internal fun JSONObject.toBillingNextCharge() = ConsumerBillingNextCharge.fromJson(consumerView("billingNextCharge"))
internal fun JSONObject.toBillingRefundRequest() = ConsumerBillingRefundRequest.fromJson(consumerView("billingRefundRequest"))
internal fun JSONObject.toPromoOfferPreview() = ConsumerPromoOfferPreview.fromJson(consumerView("promoOfferPreview"))
internal fun JSONObject.toPromoRedemption() = ConsumerPromoRedemption.fromJson(consumerView("promoRedemption"))
internal fun JSONObject.toTBankCheckoutSession() = ConsumerTBankCheckoutSession.fromJson(consumerView("tBankCheckoutSession"))
internal fun JSONObject.toBillingCatalog() = ConsumerBillingCatalog.fromJson(sharedBillingCatalog(this).consumerView("billingCatalog"))
internal fun JSONObject.toTBankCheckoutPreview() = ConsumerTBankCheckoutPreview.fromJson(sharedCheckoutPreview(this).consumerView("tBankCheckoutPreview"))
internal fun JSONObject.toPromoOffer(): PromoOffer {
    SharedPromotionRules.requiresCatalog(this)
    return ConsumerPromoOffer.fromJson(consumerView("promoOffer"))
}
internal fun parseDevices(array: JSONArray): List<AccountDevice> = List(array.length()) { index ->
    ConsumerAccountDevice.fromJson(array.getJSONObject(index).consumerView("accountDevice"))
}
internal fun JSONObject.toSession(): GovorunSession = Credentials.fromJson(consumerView("credentials")).let {
    GovorunSession(accessToken = it.accessToken, refreshToken = it.refreshToken, displayName = it.displayName,
        deviceId = it.deviceId, deviceLimit = it.deviceLimit.toInt())
}
internal fun JSONObject.toProfileSession(accessToken: String): GovorunSession =
    ConsumerProfile.fromJson(consumerView("profile")).let {
        GovorunSession(accessToken = accessToken, refreshToken = "", displayName = it.displayName,
            accountId = it.id, onlineTermsVersion = it.onlineTermsVersion, deviceId = it.currentDeviceId,
            deviceLimit = it.deviceLimit, styleAccess = it.styleAccess, starterGrant = it.starterGrant,
            promoProgram = it.promoProgram, freeAccessAvailable = it.clientFreeAccessAvailable,
            onlineConsent = it.onlineConsent, onlineService = OnlineService(OnlineServiceState.fromWire(it.onlineServiceState)),
            styleUsageAccountingAvailable = it.styleUsageAccountingAvailable, hasSyncedData = it.hasSyncedData)
    }
