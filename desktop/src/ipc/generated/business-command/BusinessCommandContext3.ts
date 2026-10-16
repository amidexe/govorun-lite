// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandPhase9 } from './BusinessCommandPhase9';
export interface BusinessCommandContext3 {
  customServer: boolean;
  enabled: boolean;
  networkAvailable: boolean;
  phase: BusinessCommandPhase9;
  profileFresh: boolean;
  refreshRequested: boolean;
  requiredCredits: number;
}
