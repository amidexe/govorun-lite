// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandNextChargeFacts } from './BusinessCommandNextChargeFacts';
import type { BusinessCommandPaymentMethodFacts } from './BusinessCommandPaymentMethodFacts';
import type { BusinessCommandRenewalIssueFacts } from './BusinessCommandRenewalIssueFacts';
export interface BusinessCommandSubscriptionFacts {
  currentPeriodEndsAt: string | null;
  nextCharge: BusinessCommandNextChargeFacts | null;
  paymentMethod: BusinessCommandPaymentMethodFacts | null;
  renewalIssue: BusinessCommandRenewalIssueFacts | null;
  renewsAutomatically: boolean;
  state: string;
}
