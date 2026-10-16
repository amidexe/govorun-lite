import { stringEnum } from '../../shared/validation/string-enum.ts';

import { validPromoProgram, type PromoProgram, type BillingStorefront } from './promo-program.ts';
import { paymentSources, validSBPBindingURL, type PaymentSource } from './payment-source.ts';
import type {
  BillingPlan as HostBillingPlan,
  BillingCheckout as HostBillingCheckout,
  BillingSnapshot as HostBillingSnapshot,
} from '../../ipc/generated/billing.ts';
export type BillingPlan = HostBillingPlan;
export interface CheckoutPreview extends Omit<HostBillingCheckout, 'paymentMethodKind' | 'availablePaymentMethods' | 'message' | 'email'> {
  message?: string | null;
  paymentMethodKind?: PaymentSource;
  availablePaymentMethods?: PaymentSource[];
  email?: string | null;
}
export type { PromoPreview } from '../../ipc/generated/billing';
import type { PromoPreview, BillingPromotionAccess as PromotionAccess } from '../../ipc/generated/billing';
export type { BillingPromotionAccess as PromotionAccess } from '../../ipc/generated/billing';
export interface BillingSnapshot extends Omit<HostBillingSnapshot, 'promoProgram' | 'storefront' | 'checkout' | 'promo' | 'promoAccess'> {
  promoProgram?: PromoProgram | null;
  storefront?: BillingStorefront;
  checkout: CheckoutPreview | null;
  promo: PromoPreview | null;
  promoAccess?: PromotionAccess | null;
}
export type BillingAction =
  | 'refresh'
  | 'resumeCheckout'
  | 'restartBinding'
  | 'previewCheckout'
  | 'choosePaymentMethod'
  | 'confirmCheckout'
  | 'dismissPreview'
  | 'previewPromo'
  | 'confirmPromo'
  | 'dismissPromo'
  | 'dismissPromoAccess'
  | 'refreshPromoAccess'
  | 'activateStarter'
  | 'cancelRenewal'
  | 'cancelPlanChange'
  | 'removeCard'
  | 'support'
  | 'legal';
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_billing');
  return value as Record<string, unknown>;
}
function strings(item: Record<string, unknown>, keys: string[], nullable = false): boolean {
  return keys.every(
    (key) =>
      (nullable && item[key] === null) ||
      (typeof item[key] === 'string' && item[key].length <= 12000),
  );
}
function flags(item: Record<string, unknown>, keys: string[]): boolean {
  return keys.every((key) => typeof item[key] === 'boolean');
}
export function billingSnapshot(value: unknown): BillingSnapshot {
  const item = record(value),
    status = record(item.status);
  if (item.promoAccess !== undefined && item.promoAccess !== null) {
    const access = record(item.promoAccess);
    if (
      !strings(access, ['id', 'message', 'deferredMessage']) ||
      !access.id ||
      !['done', 'acceptTerms', 'refresh'].includes(String(access.kind)) ||
      (access.kind === 'acceptTerms'
        ? !Number.isSafeInteger(access.termsVersion) || Number(access.termsVersion) < 1
        : access.termsVersion !== null)
    )
      throw new Error('invalid_billing');
  }
  if (
    (item.accessType !== undefined && !strings(item, ['accessType'])) ||
    (status.state !== undefined && !strings(status, ['state']))
  )
    throw new Error('invalid_billing');
  if (
    !strings(item, ['scope', 'planTitle', 'planAction']) ||
    !item.scope ||
    !strings(item, ['message', 'period', 'planDisclosure'], true) ||
    !flags(item, ['signedIn', 'busy', 'hasAccess', 'canActivateStarter', 'canPreviewCheckout',
      'showPlanIntro', 'showPlans', 'showPaymentDetails']) ||
    !Number.isSafeInteger(item.termsVersion) ||
    Number(item.termsVersion) < 1 ||
    !strings(status, ['payment', 'title']) ||
    !strings(status, ['periodEnd', 'renewalNotice', 'card'], true) ||
    !flags(status, [
      'fresh',
      'loading',
      'pendingCheckout',
      'canCancelRenewal',
      'canCancelChange',
      'canRemoveCard',
    ]) ||
    !stringEnum(item.catalogState, ['idle', 'loading', 'loaded', 'error']) ||
    !Array.isArray(item.balances) ||
    item.balances.length > 2 ||
    !item.balances.every(
      (row) => {
        const balance = record(row);
        return strings(balance, ['title', 'value']) && strings(balance, ['detail'], true) &&
          (balance.percent == null || (Number.isInteger(balance.percent) &&
            Number(balance.percent) >= 0 && Number(balance.percent) <= 100));
      },
    ) ||
    !Array.isArray(item.plans) ||
    item.plans.length > 32 ||
    !item.plans.every((row) => {
      const plan = record(row);
      return (
        strings(plan, ['id', 'title', 'price', 'period', 'limit', 'trial']) &&
        !!plan.id &&
        strings(plan, ['multiplier'], true) &&
        flags(plan, ['current'])
      );
    }) ||
    new Set(item.plans.map((row) => row.id)).size !== item.plans.length
  )
    throw new Error('invalid_billing');
  if (item.promoProgram != null && !validPromoProgram(item.promoProgram))
    throw new Error('invalid_promo_program');
  if (
    item.refreshAfterSeconds !== undefined &&
    (!Number.isInteger(item.refreshAfterSeconds) ||
      Number(item.refreshAfterSeconds) < 60 ||
      Number(item.refreshAfterSeconds) > 3600)
  )
    throw new Error('invalid_catalog_ttl');
  if (item.storefront !== undefined) {
    const storefront = record(item.storefront);
    if (
      !stringEnum(storefront.state, ['open', 'unavailable']) ||
      !strings(storefront, ['message']) ||
      (storefront.state === 'unavailable' && (item.plans.length !== 0 || item.canPreviewCheckout))
    )
      throw new Error('invalid_storefront');
  }
  if (item.checkout !== null) {
    const preview = record(item.checkout);
    if (preview.message !== undefined && !strings(preview, ['message'], true))
      throw new Error('invalid_checkout_message');
    if (
      preview.paymentMethodKind !== undefined &&
      !stringEnum(preview.paymentMethodKind, ['card', 'sbp'])
    )
      throw new Error('invalid_payment_source');
    if (
      preview.availablePaymentMethods !== undefined &&
      !paymentSources(preview.availablePaymentMethods)
    )
      throw new Error('invalid_payment_sources');
    if (
      !strings(preview, [
        'id',
        'title',
        'planTitle',
        'trialHeadline',
        'amountDueNow',
        'firstCharge',
        'renewal',
        'handoff',
        'receiptPurpose',
        'actionTitle',
        'recurringDisclosure',
        'consent',
      ]) ||
      !preview.id ||
      !strings(preview, ['emailMasked'], true) ||
      !flags(preview, [
        'emailRequired',
        'receiptContactNeeded',
        'actionEnabled',
        'hasLegalDocuments',
      ])
    )
      throw new Error('invalid_checkout');
  }
  if (item.promo !== null) {
    const promo = record(item.promo);
    if (
      !strings(promo, ['id', 'title', 'body']) ||
      !promo.id ||
      !flags(promo, ['checkoutRequired'])
    )
      throw new Error('invalid_promo');
  }
  if (
    status.bindingUrl != null &&
    (!status.pendingCheckout || !validSBPBindingURL(status.bindingUrl))
  )
    throw new Error('invalid_binding_url');
  if (
    status.paymentMethodUnavailable !== undefined &&
    typeof status.paymentMethodUnavailable !== 'boolean'
  )
    throw new Error('invalid_payment_source_status');
  if (
    status.canRestartBinding !== undefined &&
    (typeof status.canRestartBinding !== 'boolean' ||
      (status.canRestartBinding && !status.pendingCheckout))
  )
    throw new Error('invalid_binding_recovery');
  if (
    status.canResumeCheckout !== undefined &&
    (typeof status.canResumeCheckout !== 'boolean' ||
      (status.canResumeCheckout && !status.pendingCheckout))
  )
    throw new Error('invalid_checkout_resume');
  if (
    status.checkoutExpiresAt != null &&
    (typeof status.checkoutExpiresAt !== 'string' ||
      !Number.isFinite(Date.parse(status.checkoutExpiresAt)))
  )
    throw new Error('invalid_checkout_deadline');
  if (status.trialActive !== undefined && typeof status.trialActive !== 'boolean')
    throw new Error('invalid_billing');
  if (
    !status.fresh &&
    (status.canCancelRenewal ||
      status.canCancelChange ||
      status.canRemoveCard ||
      status.card !== null)
  )
    throw new Error('stale_billing_actions');
  // Access identity owns the customer-facing title; base only defines Promo quotas.
  if (item.hasAccess && item.accessType === 'promo_program') {
    return { ...item, planTitle: 'Промо' } as unknown as BillingSnapshot;
  }
  return item as unknown as BillingSnapshot;
}
