package com.govorun.lite.account

import org.json.JSONObject

typealias PromoProgram = com.govorun.lite.model.billing.PromoProgram
typealias BillingStorefront = com.govorun.lite.model.billing.BillingStorefront

internal fun JSONObject.toPromoProgram(version: String = "") =
    com.govorun.core.generated.ConsumerPromoProgram.fromJson(consumerView("promoProgram", version))
internal fun JSONObject.toBillingStorefront() =
    com.govorun.core.generated.ConsumerBillingStorefront.fromJson(consumerView("billingStorefront"))

internal fun displayPromoProgram(account: AccountState, catalog: BillingCatalog): PromoProgram =
    if (account.promoProgram.visible) account.promoProgram
    else catalog.promoProgram
