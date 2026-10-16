import type { AppearanceSnapshot, ThemeMode } from '../../ipc/generated/appearance';
export type { AppearanceSnapshot, ThemeMode } from '../../ipc/generated/appearance';
export function appearanceSnapshot(value: unknown): AppearanceSnapshot {
  const snapshot = value as Partial<AppearanceSnapshot> | null;
  if (
    !snapshot ||
    !['system', 'light', 'dark'].includes(snapshot.mode ?? '') ||
    !Number.isSafeInteger(snapshot.revision) ||
    snapshot.revision! < 0
  ) {
    throw new Error('invalid_appearance');
  }
  return snapshot as AppearanceSnapshot;
}
