// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPhase7 } from './BusinessCommandPhase7';
export interface BusinessCommandState10 {
  attempts: number;
  clock: number;
  epoch: number;
  followupUsed: boolean;
  freshFollowup: boolean;
  notBefore: number;
  phase: BusinessCommandPhase7;
  requested: boolean;
  ticket: number;
}
