// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandLength } from './BusinessCommandLength';
import type { BusinessCommandRoute } from './BusinessCommandRoute';
export interface BusinessCommandPlan {
  characters: number;
  deadlineMillis: number;
  length: BusinessCommandLength;
  prepare: boolean;
  reason: string;
  requiredCredits: number;
  route: BusinessCommandRoute;
  termsRequired: boolean;
}
