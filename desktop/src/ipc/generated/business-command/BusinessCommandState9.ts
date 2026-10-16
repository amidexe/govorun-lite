// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandLease } from './BusinessCommandLease';
export interface BusinessCommandState9 {
  clock: number;
  epoch: number;
  leases: Record<string, BusinessCommandLease>;
  scope: number | null;
  version: number;
}
