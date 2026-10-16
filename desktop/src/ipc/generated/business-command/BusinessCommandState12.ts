// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandJournal } from './BusinessCommandJournal';
import type { BusinessCommandLease2 } from './BusinessCommandLease2';
import type { BusinessCommandPass } from './BusinessCommandPass';
import type { BusinessCommandReceipt2 } from './BusinessCommandReceipt2';
export interface BusinessCommandState12 {
  available: Array<BusinessCommandPass>;
  depositReceipts: Array<string>;
  issuance: BusinessCommandJournal | null;
  pending: Array<BusinessCommandLease2>;
  receipts: Array<BusinessCommandReceipt2>;
  retiring: Array<BusinessCommandPass>;
  scopeEpoch: number | null;
}
