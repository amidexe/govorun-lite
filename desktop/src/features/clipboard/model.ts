import type { ClipboardSnapshot } from '../../ipc/generated/clipboard';
export type { ClipboardSnapshot } from '../../ipc/generated/clipboard';
export type ClipboardChange = Partial<ClipboardSnapshot>;
export function clipboardSnapshot(value: unknown): ClipboardSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_clipboard_settings');
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 2 ||
    !['preserve', 'copy'].includes(record.clipboard as string) ||
    typeof record.rememberLast !== 'boolean'
  )
    throw new Error('invalid_clipboard_settings');
  return record as unknown as ClipboardSnapshot;
}
/** «Помнить последнюю диктовку»: texts live only in host memory; null while off or before the first dictation. */
export type LastDictationSnapshot = { text: string | null; original: string | null };
export function lastDictationSnapshot(value: unknown): LastDictationSnapshot {
  const record = value as Record<string, unknown> | null;
  const text = (item: unknown) => item === null || typeof item === 'string';
  if (
    !record ||
    typeof record !== 'object' ||
    Object.keys(record).length !== 2 ||
    !text(record.text) ||
    !text(record.original)
  )
    throw new Error('invalid_last_dictation');
  return { text: record.text as string | null, original: record.original as string | null };
}
export function lastDictationCopy(value: unknown): { copied: boolean } {
  const record = value as Record<string, unknown> | null;
  if (!record || typeof record !== 'object' || typeof record.copied !== 'boolean')
    throw new Error('invalid_last_dictation');
  return { copied: record.copied };
}
