import type { SetupSnapshot, SetupProgress } from '../../ipc/generated/setup';
export type { SetupSnapshot, SetupProgress } from '../../ipc/generated/setup';
import type { RuntimeSnapshot } from '../dictation/runtime';

export const setupSteps = ['welcome', 'dictation', 'trial', 'done'] as const;
export type SetupStep = (typeof setupSteps)[number];
export const initialSetup: SetupProgress = { step: 'welcome' };

export function setupSnapshot(value: unknown): SetupSnapshot {
  const item = value as Partial<SetupSnapshot> | null;
  if (
    !item ||
    item.version !== 1 ||
    !Number.isSafeInteger(item.revision) ||
    Number(item.revision) < 0 ||
    Object.keys(item).some((key) => !['version', 'revision', 'progress'].includes(key))
  )
    throw new Error('invalid_setup');
  const progress = item.progress;
  if (
    progress !== null &&
    (!progress ||
      !setupSteps.includes(progress.step) ||
      Object.keys(progress).some((key) => key !== 'step'))
  )
    throw new Error('invalid_setup');
  // Older versions saved an optional trial; resume at readiness instead of reviving it.
  return progress?.step === 'trial'
    ? ({ ...item, progress: { step: 'dictation' } } as SetupSnapshot)
    : (item as SetupSnapshot);
}

export function dictationReady(runtime: RuntimeSnapshot | null) {
  const ready = runtime?.readiness;
  return (
    runtime?.phase === 'idle' &&
    runtime.issue !== 'captureUnavailable' &&
    !!ready &&
    ['ready', 'standby'].includes(ready.model) &&
    ready.microphone &&
    ready.microphoneAvailable !== false &&
    ready.accessibility &&
    ready.hotkey
  );
}
