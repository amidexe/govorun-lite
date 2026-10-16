// @generated from shared/core serde JSON Schema; do not edit.
package com.govorun.core.generated

data class BillingSnapshot(
    val `accessType`: String,
    val `balances`: List<BillingBalance>,
    val `busy`: Boolean,
    val `canActivateStarter`: Boolean,
    val `canPreviewCheckout`: Boolean,
    val `catalogState`: String,
    val `checkout`: BillingCheckout?,
    val `hasAccess`: Boolean,
    val `message`: String?,
    val `period`: String?,
    val `planAction`: String,
    val `planDisclosure`: String?,
    val `planTitle`: String,
    val `plans`: List<BillingPlan>,
    val `promo`: PromoPreview?,
    val `promoAccess`: BillingPromotionAccess?,
    val `promoProgram`: PromoProgram?,
    val `refreshAfterSeconds`: Long,
    val `scope`: String,
    val `showPaymentDetails`: Boolean,
    val `showPlanIntro`: Boolean,
    val `showPlans`: Boolean,
    val `signedIn`: Boolean,
    val `status`: BillingStatus,
    val `storefront`: BillingStorefront,
    val `termsVersion`: Int
)
