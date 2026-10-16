// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDay } from './BusinessCommandDay';
import type { BusinessCommandTotal } from './BusinessCommandTotal';
export interface BusinessCommandResponse {
  cursor: number;
  dayWindowStart: string;
  days: Array<BusinessCommandDay>;
  generation: number;
  schemaVersion: number;
  snapshot: boolean;
  total: BusinessCommandTotal;
}
