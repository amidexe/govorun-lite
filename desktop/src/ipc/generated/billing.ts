// @generated from shared/core serde JSON Schema; do not edit.
export interface BillingBalance {
  detail: string | null;
  percent: number | null;
  title: string;
  value: string;
}
export interface BillingCheckout {
  actionEnabled: boolean;
  actionTitle: string;
  amountDueNow: string;
  availablePaymentMethods: Array<string>;
  consent: string;
  email: string | null;
  emailMasked: string | null;
  emailRequired: boolean;
  firstCharge: string;
  handoff: string;
  hasLegalDocuments: boolean;
  id: string;
  message: string | null;
  paymentMethodKind: string;
  planTitle: string;
  receiptContactNeeded: boolean;
  receiptPurpose: string;
  recurringDisclosure: string;
  renewal: string;
  title: string;
  trialHeadline: string;
}
export interface BillingPlan {
  current: boolean;
  id: string;
  limit: string;
  multiplier: string | null;
  period: string;
  price: string;
  title: string;
  trial: string;
}
export interface BillingPromotionAccess {
  deferredMessage: string;
  id: string;
  kind: BillingPromotionAccessKind;
  message: string;
  termsVersion: number | null;
}
export type BillingPromotionAccessKind = "done" | "acceptTerms" | "refresh";
export interface BillingSnapshot {
  accessType: string;
  balances: Array<BillingBalance>;
  busy: boolean;
  canActivateStarter: boolean;
  canPreviewCheckout: boolean;
  catalogState: string;
  checkout: BillingCheckout | null;
  hasAccess: boolean;
  message: string | null;
  period: string | null;
  planAction: string;
  planDisclosure: string | null;
  planTitle: string;
  plans: Array<BillingPlan>;
  promo: PromoPreview | null;
  promoAccess: BillingPromotionAccess | null;
  promoProgram: PromoProgram | null;
  refreshAfterSeconds: number;
  scope: string;
  showPaymentDetails: boolean;
  showPlanIntro: boolean;
  showPlans: boolean;
  signedIn: boolean;
  status: BillingStatus;
  storefront: BillingStorefront;
  termsVersion: number;
}
export interface BillingStatus {
  bindingUrl: string | null;
  canCancelChange: boolean;
  canCancelRenewal: boolean;
  canRemoveCard: boolean;
  canRestartBinding: boolean;
  canResumeCheckout: boolean;
  card: string | null;
  checkoutExpiresAt: string | null;
  fresh: boolean;
  loading: boolean;
  payment: string;
  paymentMethodUnavailable: boolean;
  pendingCheckout: boolean;
  periodEnd: string | null;
  renewalNotice: string | null;
  state: string;
  title: string;
  trialActive: boolean;
}
export interface BillingStorefront {
  message: string;
  state: string;
}
export interface PromoPreview {
  body: string;
  bonusDays: number | null;
  checkoutRequired: boolean;
  id: string;
  title: string;
}
export interface PromoProgram {
  accessDays: number;
  accessMode: string;
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
