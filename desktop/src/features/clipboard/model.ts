import type { ClipboardSnapshot } from '../../ipc/generated/clipboard';
export type { ClipboardSnapshot } from '../../ipc/generated/clipboard';
export function clipboardSnapshot(value: unknown): ClipboardSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_clipboard_settings');
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 1 ||
    !['preserve', 'copy'].includes(record.clipboard as string)
  )
    throw new Error('invalid_clipboard_settings');
  return record as unknown as ClipboardSnapshot;
}
