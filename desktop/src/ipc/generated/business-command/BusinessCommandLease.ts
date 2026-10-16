// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPhase5 } from './BusinessCommandPhase5';
export interface BusinessCommandLease {
  notBefore: number;
  owner: string | null;
  phase: BusinessCommandPhase5;
  version: number;
}
