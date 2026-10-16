// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandChanges } from './BusinessCommandChanges';
import type { BusinessCommandPhase } from './BusinessCommandPhase';
export interface BusinessCommandFlow3 {
  conflicts: number;
  phase: BusinessCommandPhase;
  sent: BusinessCommandChanges;
}
