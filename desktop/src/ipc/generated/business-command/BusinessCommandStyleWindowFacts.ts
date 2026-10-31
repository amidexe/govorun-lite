// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandLedger } from './BusinessCommandLedger';
import type { BusinessCommandProductWindow } from './BusinessCommandProductWindow';
import type { BusinessCommandScope3 } from './BusinessCommandScope3';
export interface BusinessCommandStyleWindowFacts {
  accessActive: boolean;
  accessType: string;
  balanceKind: string;
  dailyLimit: number;
  dailyResetPassed: boolean;
  entitlementActive: boolean;
  freeAccessAvailable: boolean;
  issuanceInFlight: boolean;
  localBound: boolean;
  localKnown: boolean;
  pendingCredits: number;
  pendingUsageToday: number;
  pendingUsageTotal: number;
  presentationAccess: unknown;
  processingReady: boolean;
  products: Array<BusinessCommandProductWindow>;
  profileFresh: boolean;
  readyCredits: number;
  recoveryInFlight: boolean;
  remainingToday: number;
  remainingTotal: number;
  reserveReturn: boolean;
  signedIn: boolean;
  starterAllocated: boolean;
  totalLimit: number;
  usageAccounting: boolean;
  usageFresh: boolean;
  usageLedger: BusinessCommandLedger | null;
  usageScope: BusinessCommandScope3 | null;
}
