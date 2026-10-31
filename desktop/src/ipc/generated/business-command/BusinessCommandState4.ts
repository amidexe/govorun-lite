// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandState4 {
  backoff: number;
  failures: number;
  generation: number;
  lastFailure: number | null;
  paused: boolean;
  trialActive: boolean;
  trialAt: number;
}
