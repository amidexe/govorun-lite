// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandSubscriptionFacts } from './BusinessCommandSubscriptionFacts';
export interface BusinessCommandOverviewFacts {
  catalogTrialDays: number | null;
  checkoutPending: boolean;
  epoch: number;
  now: string;
  paymentUpdating: boolean;
  signedIn: boolean;
  statusFresh: boolean;
  statusUpdating: boolean;
  subscription: BusinessCommandSubscriptionFacts | null;
}
