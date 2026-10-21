// @generated from shared/core serde JSON Schema; do not edit.
export interface BenchmarkDevice {
  backend: string;
  failureCode: string | null;
  failureDetail: string | null;
  id: string;
  name: string;
  result: BenchmarkResult | null;
  stage: string;
  state: string;
}
export interface BenchmarkResult {
  cold: BenchmarkTiming;
  loadMs: number;
  longAudioMs: number;
  longer: BenchmarkTiming;
  model: string;
  platform: string;
  requestedBackend: string;
  shortAudioMs: number;
  suite: string;
  warm: BenchmarkTiming;
}
export interface BenchmarkSnapshot {
  activeDeviceId: string | null;
  devices: Array<BenchmarkDevice>;
  failureCode: string | null;
  report: string;
  stage: string;
  state: string;
}
export interface BenchmarkTiming {
  backend: string;
  milliseconds: number;
}
