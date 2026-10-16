// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandState4 {
  backoff: number;
  freshDemand: boolean;
  generation: number;
  hasOpened: boolean;
  lastFailure: number | null;
  lastRecordingProbe: number | null;
  paused: boolean;
  probeActive: boolean;
  probeAt: number;
  protectedUntil: number;
  remaining: number;
}
