// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandDeviceDay {
  date: string;
  recognitionErrors: number;
  recognitionRtfBuckets: Array<number>;
  recognitions: number;
  revision: number;
  speechSeconds: number;
  styleAttempts: number;
  styleFailures: number;
  styleSlow: number;
  words: number;
}
