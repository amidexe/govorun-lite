//! Build-only dependent display defaults; no account/payment decisions.
#[cfg(feature = "schema")]
pub fn constructor_defaults() -> serde_json::Value { serde_json::json!({
    "ConsumerBillingPlan": {"offerId":"code", "billingPeriodLabel":"\"$periodDays дней\""},
    "ConsumerBillingCatalog": {"tBankPreviewEnabled":"tBankEnabled"},
    "ConsumerTBankCheckoutPreview": {"offerId":"\"$planCode.monthly\""},
    "ConsumerTBankCheckoutSession": {"offerId":"\"$planCode.monthly\""},
    "ConsumerPromoOffer": {"offerId":"\"$planCode.monthly\""}
}) }

#[cfg(feature = "schema")]
pub fn field_order() -> serde_json::Value { serde_json::json!({
    "ConsumerStyleAccess": ["type", "state", "startsAt", "endsAt", "dailyResetsAt", "periodResetsAt", "dailyLimit", "totalLimit", "issuedToday", "issuedTotal", "remainingToday", "remainingTotal", "quotaUnit", "charactersPerCredit", "maximumCreditsPerRequest", "stylizationProtocol", "planCode", "balanceKind", "usageBalance", "reserveReturnProtocol", "nextPeriodAt", "periodResetCondition", "accessEndsAt", "issuanceCountersObserved"],
    "ConsumerOnlineConsent": ["acceptedVersion", "acceptedAt", "active"],
    "ConsumerAccountDevice": ["id", "platform", "displayName", "activatedAt", "lastSeenAt", "current"],
    "ConsumerStarterGrant": ["state", "creditLimit", "remainingToIssue", "activationAvailable", "expires", "balanceKind", "trialOffer", "eligible"],
    "ConsumerPromoProgram": ["campaignId", "programVersion", "state", "enrollmentOpen", "activationAvailable", "participated", "accessMode", "accessDays", "accessEndsAt", "endsAt", "dailyLimit", "periodLimit", "periodDays", "message", "visible", "canActivate"],
    "ConsumerBillingStorefront": ["state", "message"],
    "ConsumerBillingPaymentMethod": ["state", "lastFour", "canRemove", "kind", "available"],
    "ConsumerPromoAccessDays": ["days", "planCode", "planName", "startsAt", "endsAt"],
    "ConsumerPromoBillingPeriod": ["number", "standardPriceKopeks", "payablePriceKopeks", "discountAmountKopeks"],
    "ConsumerPromoBonus": ["credits", "validDays"],
    "ConsumerPromoOffer": ["planCode", "planName", "currency", "billingPeriods", "followingPriceKopeks", "followingPeriodDays", "bonus", "confirmationTitle", "confirmationBody", "createsSubscription", "checkoutRequired", "grantsAccessOnConfirmation", "catalogVersion", "offerId", "accessDays"],
    "ConsumerPromoOfferPreview": ["previewId", "previewExpiresAt", "offer"],
    "ConsumerPromoRedemption": ["redemptionId", "redeemedAt", "offer"],
    "ConsumerBillingPlan": ["code", "name", "priceKopeks", "dailyLimit", "periodLimit", "periodDays", "recommended", "ruStoreProductId", "offerId", "billingPeriodLabel", "clientMultiplierCopy", "clientLimitCopy"],
    "ConsumerBillingCatalog": ["schemaVersion", "catalogVersion", "publishedAt", "refreshAfterSeconds", "currency", "ruStoreEnabled", "tBankEnabled", "tBankPreviewEnabled", "trialOffer", "starterGrant", "plans", "storefront", "programControlVersion", "promoProgram", "clientTrialDays"],
    "ConsumerStarterGrantOffer": ["clientOfferVisible", "state", "creditLimit", "requiresAuthentication", "requiresCode", "requiresPaymentMethod", "expires"],
    "ConsumerBillingTrialOffer": ["state", "eligible", "durationDays", "requiresVerifiedPaymentMethod", "checkoutOpenGrantsAccess", "requiresLocalReserveEmpty"],
    "ConsumerBillingStatus": ["subscription", "starterGrant"],
    "ConsumerBillingSubscription": ["state", "planCode", "offerId", "provider", "currentPeriodEndsAt", "renewsAutomatically", "pendingPlanCode", "pendingOfferId", "changeEffectiveAt", "billingSubjectId", "serviceAccessState", "suspensionReason", "suspendedUntil", "refundRequest", "renewalIssue", "nextCharge", "paymentMethod"],
    "ConsumerBillingRenewalIssue": ["state", "resolutionKind", "chargeKind", "paymentState", "amountKopeks", "currency", "paymentMethodSaved", "attemptCount", "maximumAttempts", "occurredAt", "nextAttemptAt", "accessEndsAt"],
    "ConsumerBillingNextCharge": ["amountKopeks", "currency", "chargeAt", "noticeAt"],
    "ConsumerBillingRefundRequest": ["id", "state", "reasonCode", "requestedAt", "decisionCode", "decisionMode", "approvedAmountKopeks", "currency", "renewalStopped", "currentAccessEndsAt"],
    "ConsumerBillingPlanChange": ["mode", "productId", "targetOfferId", "effectiveAt", "carryoverLimit", "clientCheckout"],
    "ConsumerTBankCheckoutPreview": ["clientCheckoutPreview", "paymentMethodKind", "availablePaymentMethods", "checkoutAvailable", "catalogVersion", "planCode", "offerId", "purchaseKind", "trialDays", "freeAccessSource", "freeAccessEndsAt", "firstChargeAt", "priceKopeks", "periodDays", "currency", "serverConfirmationRequired", "receiptContactState", "receiptEmailMasked", "receiptEmail", "receiptContactToken", "amountDueNowKopeks", "firstChargeAmountKopeks", "renewalChargeAt", "renewalAmountKopeks", "automaticRenewal", "cancellationMethod", "subscriptionConsentInitiallyAccepted", "subscriptionConsentToken", "legalDocuments", "promotion", "fromPlanCode", "checkoutIntentToken", "servicePeriodEndsAt", "firstChargeDelaySeconds"],
    "ConsumerCheckoutPreviewAction": ["canConfirm", "confirmationPath", "receiptContactRequired"],
    "ConsumerTBankLegalDocument": ["version", "url"],
    "ConsumerTBankLegalDocuments": ["subscriptionTerms", "userAgreement", "refundPolicy", "privacyNotice", "receiptDataConsent"],
    "ConsumerTBankCheckoutSession": ["paymentMethodKind", "catalogVersion", "checkoutSessionId", "state", "planCode", "offerId", "purchaseKind", "trialDays", "freeAccessSource", "freeAccessEndsAt", "firstChargeAt", "priceKopeks", "periodDays", "currency", "browserReturnGrantsAccess", "serverConfirmationRequired", "paymentUrl", "firstChargeDelaySeconds", "canRestartBinding", "cancellationPending", "expiresInSeconds", "clientCheckout"],
    "ConsumerCheckoutPresentation": ["disposition", "canCancel", "resumeUrl", "expiresInSeconds", "message", "accessRefreshPending", "launchAction"],
    "ConsumerCheckoutMessageUpdate": ["replace", "message", "refreshStatus", "refreshAccess"],
    "ConsumerBillingCheckoutPresentation": ["settled", "scheduled"],
    "ConsumerProfile": ["id", "displayName", "currentDeviceId", "deviceLimit", "onlineTermsVersion", "styleAccess", "onlineConsent", "onlineServiceState", "starterGrant", "promoProgram", "clientFreeAccessAvailable", "styleUsageAccountingAvailable", "hasSyncedData"],
    "ConsumerStyleUsageBalance": ["scopeId", "revision", "day", "serverTime", "consumedToday", "consumedTotal", "remainingToday", "remainingTotal", "deviceDays", "settlementScopeIds", "dailySettlementScopeIds", "deviceRequests", "version"],
    "ConsumerStyleRequestCount": ["count", "credits"]
}) }
