// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDay } from './BusinessCommandDay';
import type { BusinessCommandTotal } from './BusinessCommandTotal';
export interface BusinessCommandSnapshot {
  cursor: number;
  days: Array<BusinessCommandDay>;
  generation: number;
  total: BusinessCommandTotal | null;
}
