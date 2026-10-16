// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

enum class BusinessCommandKind(val wireValue: String) {
    CREDENTIALS("credentials"),
    PROFILE("profile"),
    STYLE_ACCESS("styleAccess"),
    ONLINE_CONSENT("onlineConsent"),
    ACCOUNT_DEVICE("accountDevice"),
    STYLE_USAGE_BALANCE("styleUsageBalance"),
    STARTER_GRANT("starterGrant"),
    STARTER_GRANT_OFFER("starterGrantOffer"),
    BILLING_TRIAL_OFFER("billingTrialOffer"),
    BILLING_CATALOG("billingCatalog"),
    BILLING_STATUS("billingStatus"),
    BILLING_PLAN("billingPlan"),
    BILLING_SUBSCRIPTION("billingSubscription"),
    BILLING_RENEWAL_ISSUE("billingRenewalIssue"),
    BILLING_NEXT_CHARGE("billingNextCharge"),
    BILLING_REFUND_REQUEST("billingRefundRequest"),
    BILLING_PLAN_CHANGE("billingPlanChange"),
    BILLING_PAYMENT_METHOD("billingPaymentMethod"),
    BILLING_STOREFRONT("billingStorefront"),
    T_BANK_CHECKOUT_PREVIEW("tBankCheckoutPreview"),
    T_BANK_CHECKOUT_SESSION("tBankCheckoutSession"),
    T_BANK_LEGAL_DOCUMENT("tBankLegalDocument"),
    T_BANK_LEGAL_DOCUMENTS("tBankLegalDocuments"),
    CHECKOUT_PRESENTATION("checkoutPresentation"),
    BILLING_CHECKOUT_PRESENTATION("billingCheckoutPresentation"),
    PROMO_PROGRAM("promoProgram"),
    PROMO_OFFER("promoOffer"),
    PROMO_OFFER_PREVIEW("promoOfferPreview"),
    PROMO_REDEMPTION("promoRedemption");
    companion object {
        fun fromWire(value: String): BusinessCommandKind? =
            entries.firstOrNull { it.wireValue == value }
    }
}
