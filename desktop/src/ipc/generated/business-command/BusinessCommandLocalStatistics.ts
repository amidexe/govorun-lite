// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDay } from './BusinessCommandDay';
import type { BusinessCommandTotal } from './BusinessCommandTotal';
export interface BusinessCommandLocalStatistics {
  days: Array<BusinessCommandDay>;
  total: BusinessCommandTotal;
}
