// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandChanges } from './BusinessCommandChanges';
import type { BusinessCommandValues } from './BusinessCommandValues';
export interface BusinessCommandState3 {
  baseline: BusinessCommandValues | null;
  pending: BusinessCommandChanges;
  schemaVersion: number;
}
