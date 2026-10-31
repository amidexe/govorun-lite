// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPhase2 } from './BusinessCommandPhase2';
import type { BusinessCommandPlan } from './BusinessCommandPlan';
export interface BusinessCommandState6 {
  digest: boolean;
  phase: BusinessCommandPhase2;
  plan: BusinessCommandPlan | null;
  product: boolean;
}
