import type { RuntimeSnapshot } from '../../features/dictation/runtime';
import { runtimeStatus } from '../../features/dictation/readiness.ts';

// Improvement outcomes belong to its own card; the header describes dictation.
export function homeStatus(snapshot: RuntimeSnapshot | null) {
  return runtimeStatus(snapshot?.phase === 'fallback' ? { ...snapshot, phase: 'idle' } : snapshot);
}
