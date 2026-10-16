// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPhase8 } from './BusinessCommandPhase8';
export interface BusinessCommandState11 {
  attempts: number;
  clock: number;
  epoch: number;
  explicitDemand: boolean;
  freshFollowup: boolean;
  localNotBefore: number;
  pendingDemand: boolean;
  phase: BusinessCommandPhase8;
  requested: boolean;
  serverNotBefore: number;
  ticket: number;
}
