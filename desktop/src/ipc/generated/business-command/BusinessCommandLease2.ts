// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPass } from './BusinessCommandPass';
export interface BusinessCommandLease2 {
  id: string;
  passes: Array<BusinessCommandPass>;
  scopeEpoch: number | null;
}
