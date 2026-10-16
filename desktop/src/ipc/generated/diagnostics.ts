// @generated from shared/core serde JSON Schema; do not edit.
export interface DiagnosticsRow {
  category: string;
  compact?: string;
  date: string;
  details: string | null;
  id: string;
  level: string;
  summary: string;
  timing: string | null;
}
export interface DiagnosticsSnapshot {
  events: Array<DiagnosticsRow>;
  extendedEnabled?: boolean;
  extendedEvents?: Array<DiagnosticsRow>;
  maximumEvents: number;
  nativeAvailable?: boolean;
}
