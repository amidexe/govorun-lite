import type { RecordingDiagnosticState } from '../../ipc/generated/recording-diagnostics';
export type { RecordingDiagnosticState } from '../../ipc/generated/recording-diagnostics';
export function recordingDiagnosticState(value: unknown): RecordingDiagnosticState {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Invalid support state');
  const row = value as Record<string, unknown>;
  if (
    Object.keys(row).sort().join(',') !== 'enabled,hasFiles,recording' ||
    typeof row.enabled !== 'boolean' ||
    typeof row.recording !== 'boolean' ||
    typeof row.hasFiles !== 'boolean'
  )
    throw new Error('Invalid support state');
  return row as unknown as RecordingDiagnosticState;
}
