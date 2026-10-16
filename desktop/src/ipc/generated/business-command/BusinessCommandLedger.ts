// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandRequestCount } from './BusinessCommandRequestCount';
import type { BusinessCommandReservation } from './BusinessCommandReservation';
export interface BusinessCommandLedger {
  acknowledged: Record<string, Record<string, number>>;
  pending: Record<string, BusinessCommandReservation>;
  requests: Record<string, Record<string, BusinessCommandRequestCount>>;
  totals: Record<string, Record<string, number>>;
}
