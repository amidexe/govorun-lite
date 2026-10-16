// @generated from shared/core serde JSON Schema; do not edit.
export interface ConsumerAccountDevice {
  activatedAt: string;
  current: boolean;
  displayName: string;
  id: string;
  lastSeenAt: string;
  platform: string;
}
export interface ConsumerBillingCatalog {
  catalogVersion: string;
  clientTrialDays: number;
  currency: string;
  plans: Array<ConsumerBillingPlan>;
  programControlVersion: string;
  promoProgram: ConsumerPromoProgram;
  publishedAt: string;
  refreshAfterSeconds: number;
  ruStoreEnabled: boolean;
  schemaVersion: number;
  starterGrant: ConsumerStarterGrantOffer;
  storefront: ConsumerBillingStorefront;
  tBankEnabled: boolean;
  tBankPreviewEnabled: boolean;
  trialOffer: ConsumerBillingTrialOffer;
}
export interface ConsumerBillingCheckoutPresentation {
  scheduled: ConsumerCheckoutMessageUpdate;
  settled: ConsumerCheckoutMessageUpdate;
}
export interface ConsumerBillingNextCharge {
  amountKopeks: number;
  chargeAt: string;
  currency: string;
  noticeAt: string;
}
export interface ConsumerBillingPaymentMethod {
  available: boolean | null;
  canRemove: boolean;
  kind: string;
  lastFour: string | null;
  state: string;
}
export interface ConsumerBillingPlan {
  billingPeriodLabel: string;
  clientLimitCopy: string;
  clientMultiplierCopy: string | null;
  code: string;
  dailyLimit: number;
  name: string;
  offerId: string;
  periodDays: number;
  periodLimit: number;
  priceKopeks: number;
  recommended: boolean;
  ruStoreProductId: string;
}
export interface ConsumerBillingPlanChange {
  carryoverLimit: number;
  clientCheckout: ConsumerCheckoutPresentation | null;
  effectiveAt: string;
  mode: string;
  productId: string;
  targetOfferId: string;
}
export interface ConsumerBillingRefundRequest {
  approvedAmountKopeks: number;
  currency: string;
  currentAccessEndsAt: string;
  decisionCode: string;
  decisionMode: string;
  id: string;
  reasonCode: string;
  renewalStopped: boolean;
  requestedAt: string;
  state: string;
}
export interface ConsumerBillingRenewalIssue {
  accessEndsAt: string;
  amountKopeks: number;
  attemptCount: number;
  chargeKind: string;
  currency: string;
  maximumAttempts: number;
  nextAttemptAt: string;
  occurredAt: string;
  paymentMethodSaved: boolean;
  paymentState: string;
  resolutionKind: string;
  state: string;
}
export interface ConsumerBillingStatus {
  starterGrant: ConsumerStarterGrant;
  subscription: ConsumerBillingSubscription;
}
export interface ConsumerBillingStorefront {
  message: string;
  state: string;
}
export interface ConsumerBillingSubscription {
  billingSubjectId: string;
  changeEffectiveAt: string;
  currentPeriodEndsAt: string;
  nextCharge: ConsumerBillingNextCharge | null;
  offerId: string;
  paymentMethod: ConsumerBillingPaymentMethod | null;
  pendingOfferId: string;
  pendingPlanCode: string;
  planCode: string;
  provider: string;
  refundRequest: ConsumerBillingRefundRequest | null;
  renewalIssue: ConsumerBillingRenewalIssue | null;
  renewsAutomatically: boolean;
  serviceAccessState: string;
  state: string;
  suspendedUntil: string;
  suspensionReason: string;
}
export interface ConsumerBillingTrialOffer {
  checkoutOpenGrantsAccess: boolean;
  durationDays: number;
  eligible: boolean;
  requiresLocalReserveEmpty: boolean;
  requiresVerifiedPaymentMethod: boolean;
  state: string;
}
export interface ConsumerCheckoutMessageUpdate {
  message: string | null;
  refreshAccess: boolean;
  refreshStatus: boolean;
  replace: boolean;
}
export interface ConsumerCheckoutPresentation {
  accessRefreshPending: boolean;
  canCancel: boolean;
  disposition: string;
  expiresInSeconds: number | null;
  launchAction: string | null;
  message: string;
  resumeUrl: string | null;
}
export interface ConsumerCheckoutPreviewAction {
  canConfirm: boolean;
  confirmationPath: string;
  receiptContactRequired: boolean;
}
export interface ConsumerOnlineConsent {
  acceptedAt: string;
  acceptedVersion: number;
  active: boolean;
}
export interface ConsumerProfile {
  clientFreeAccessAvailable: boolean;
  currentDeviceId: string;
  deviceLimit: number;
  displayName: string;
  hasSyncedData: boolean | null;
  id: string;
  onlineConsent: ConsumerOnlineConsent;
  onlineServiceState: string;
  onlineTermsVersion: number;
  promoProgram: ConsumerPromoProgram;
  starterGrant: ConsumerStarterGrant;
  styleAccess: ConsumerStyleAccess;
  styleUsageAccountingAvailable: boolean;
}
export interface ConsumerPromoAccessDays {
  days: number;
  endsAt: string;
  planCode: string;
  planName: string;
  startsAt: string;
}
export interface ConsumerPromoBillingPeriod {
  discountAmountKopeks: number;
  number: number;
  payablePriceKopeks: number;
  standardPriceKopeks: number;
}
export interface ConsumerPromoBonus {
  credits: number;
  validDays: number;
}
export interface ConsumerPromoOffer {
  accessDays: ConsumerPromoAccessDays | null;
  billingPeriods: Array<ConsumerPromoBillingPeriod>;
  bonus: ConsumerPromoBonus | null;
  catalogVersion: string;
  checkoutRequired: boolean;
  confirmationBody: string;
  confirmationTitle: string;
  createsSubscription: boolean;
  currency: string;
  followingPeriodDays: number;
  followingPriceKopeks: number;
  grantsAccessOnConfirmation: boolean;
  offerId: string;
  planCode: string;
  planName: string;
}
export interface ConsumerPromoOfferPreview {
  offer: ConsumerPromoOffer;
  previewExpiresAt: string;
  previewId: string;
}
export interface ConsumerPromoProgram {
  accessDays: number;
  accessEndsAt: string | null;
  accessMode: string;
  activationAvailable: boolean;
  campaignId: string;
  canActivate: boolean;
  dailyLimit: number;
  endsAt: string | null;
  enrollmentOpen: boolean;
  message: string;
  participated: boolean;
  periodDays: number;
  periodLimit: number;
  programVersion: string;
  state: string;
  visible: boolean;
}
export interface ConsumerPromoRedemption {
  offer: ConsumerPromoOffer;
  redeemedAt: string;
  redemptionId: string;
}
export interface ConsumerStarterGrant {
  activationAvailable: boolean;
  balanceKind: string;
  creditLimit: number;
  eligible: boolean;
  expires: boolean;
  remainingToIssue: number;
  state: string;
  trialOffer: ConsumerBillingTrialOffer | null;
}
export interface ConsumerStarterGrantOffer {
  clientOfferVisible: boolean;
  creditLimit: number;
  expires: boolean;
  requiresAuthentication: boolean;
  requiresCode: boolean;
  requiresPaymentMethod: boolean;
  state: string;
}
export interface ConsumerStyleAccess {
  accessEndsAt: string;
  balanceKind: string;
  charactersPerCredit: number;
  dailyLimit: number;
  dailyResetsAt: string;
  endsAt: string;
  issuanceCountersObserved: boolean;
  issuedToday: number;
  issuedTotal: number;
  maximumCreditsPerRequest: number;
  nextPeriodAt: string;
  periodResetCondition: string;
  periodResetsAt: string;
  planCode: string;
  quotaUnit: string | null;
  remainingToday: number;
  remainingTotal: number;
  reserveReturnProtocol: string | null;
  startsAt: string;
  state: string;
  stylizationProtocol: string | null;
  totalLimit: number;
  type: string;
  usageBalance: ConsumerStyleUsageBalance | null;
}
export interface ConsumerStyleRequestCount {
  count: number;
  credits: number;
}
export interface ConsumerStyleUsageBalance {
  consumedToday: number;
  consumedTotal: number;
  dailySettlementScopeIds: Array<string>;
  day: string;
  deviceDays: Record<string, number>;
  deviceRequests: Record<string, ConsumerStyleRequestCount>;
  remainingToday: number;
  remainingTotal: number;
  revision: number;
  scopeId: string;
  serverTime: string;
  settlementScopeIds: Array<string>;
  version: number;
}
export interface ConsumerTBankCheckoutPreview {
  amountDueNowKopeks: number;
  automaticRenewal: boolean;
  availablePaymentMethods: Array<string>;
  cancellationMethod: string;
  catalogVersion: string;
  checkoutAvailable: boolean;
  checkoutIntentToken: string;
  clientCheckoutPreview: ConsumerCheckoutPreviewAction | null;
  currency: string;
  firstChargeAmountKopeks: number;
  firstChargeAt: string;
  firstChargeDelaySeconds: number | null;
  freeAccessEndsAt: string;
  freeAccessSource: string;
  fromPlanCode: string;
  legalDocuments: ConsumerTBankLegalDocuments;
  offerId: string;
  paymentMethodKind: string;
  periodDays: number;
  planCode: string;
  priceKopeks: number;
  promotion: ConsumerPromoOffer | null;
  purchaseKind: string;
  receiptContactState: string;
  receiptContactToken: string;
  receiptEmail: string;
  receiptEmailMasked: string;
  renewalAmountKopeks: number;
  renewalChargeAt: string;
  serverConfirmationRequired: boolean;
  servicePeriodEndsAt: string;
  subscriptionConsentInitiallyAccepted: boolean;
  subscriptionConsentToken: string;
  trialDays: number;
}
export interface ConsumerTBankCheckoutSession {
  browserReturnGrantsAccess: boolean;
  canRestartBinding: boolean;
  cancellationPending: boolean;
  catalogVersion: string;
  checkoutSessionId: string;
  clientCheckout: ConsumerCheckoutPresentation | null;
  currency: string;
  expiresInSeconds: number | null;
  firstChargeAt: string;
  firstChargeDelaySeconds: number | null;
  freeAccessEndsAt: string;
  freeAccessSource: string;
  offerId: string;
  paymentMethodKind: string;
  paymentUrl: string;
  periodDays: number;
  planCode: string;
  priceKopeks: number;
  purchaseKind: string;
  serverConfirmationRequired: boolean;
  state: string;
  trialDays: number;
}
export interface ConsumerTBankLegalDocument {
  url: string;
  version: string;
}
export interface ConsumerTBankLegalDocuments {
  privacyNotice: ConsumerTBankLegalDocument;
  receiptDataConsent: ConsumerTBankLegalDocument;
  refundPolicy: ConsumerTBankLegalDocument;
  subscriptionTerms: ConsumerTBankLegalDocument;
  userAgreement: ConsumerTBankLegalDocument;
}
export interface Credentials {
  accessToken: string;
  deviceId: string;
  deviceLimit: number;
  displayName: string;
  refreshToken: string;
}
