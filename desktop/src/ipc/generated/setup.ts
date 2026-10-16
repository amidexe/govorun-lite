// @generated from shared/core serde JSON Schema; do not edit.
export interface SetupProgress {
  step: SetupStep;
}
export interface SetupSnapshot {
  progress: SetupProgress | null;
  revision: number;
  version: number;
}
export type SetupStep = "welcome" | "dictation" | "trial" | "done";
