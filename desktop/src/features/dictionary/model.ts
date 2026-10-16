import type { DictionarySnapshot } from '../../ipc/generated/dictionary';
export type { DictionaryEntry, DictionarySnapshot } from '../../ipc/generated/dictionary';

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_snapshot');
  return value as Record<string, unknown>;
}
export function dictionarySnapshot(value: unknown): DictionarySnapshot {
  const item = record(value);
  if (
    typeof item.enabled !== 'boolean' ||
    typeof item.revision !== 'string' || !item.revision || item.revision.length > 128 ||
    !Array.isArray(item.entries) ||
    item.entries.length > 5_000 ||
    !item.entries.every((entry) => {
      const row = record(entry);
      return (
        typeof row.id === 'string' &&
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(row.id) &&
        typeof row.from === 'string' &&
        row.from.length > 0 &&
        row.from.length <= 500 &&
        typeof row.to === 'string' &&
        row.to.length <= 500
      );
    }) ||
    new Set(item.entries.map((entry) => entry.id)).size !== item.entries.length
  )
    throw new Error('invalid_dictionary');
  return item as unknown as DictionarySnapshot;
}
