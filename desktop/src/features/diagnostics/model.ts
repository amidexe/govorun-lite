import type {
  DiagnosticsRow as DiagnosticEvent,
  DiagnosticsSnapshot,
} from '../../ipc/generated/diagnostics';
export type {
  DiagnosticsRow as DiagnosticEvent,
  DiagnosticsSnapshot,
} from '../../ipc/generated/diagnostics';
function record(value: unknown, keys: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !keys.includes(key))
  )
    throw new Error('invalid_diagnostics');
  return value as Record<string, unknown>;
}
const text = (value: unknown, limit: number): value is string =>
  typeof value === 'string' && value.length <= limit;
export function diagnosticsSnapshot(value: unknown): DiagnosticsSnapshot {
  const item = record(value, [
    'maximumEvents',
    'events',
    'extendedEvents',
    'extendedEnabled',
    'nativeAvailable',
  ]);
  if ('extendedEnabled' in item && typeof item.extendedEnabled !== 'boolean')
    throw new Error('invalid_diagnostics');
  if ('nativeAvailable' in item && typeof item.nativeAvailable !== 'boolean')
    throw new Error('invalid_diagnostics');
  if (item.maximumEvents !== 200 || !Array.isArray(item.events) || item.events.length > 200)
    throw new Error('invalid_diagnostics');
  if (
    'extendedEvents' in item &&
    (!Array.isArray(item.extendedEvents) || item.extendedEvents.length > 200)
  )
    throw new Error('invalid_diagnostics');
  const rows = [...item.events, ...((item.extendedEvents as DiagnosticEvent[] | undefined) ?? [])];
  for (const value of rows) {
    const event = record(value, [
      'id',
      'date',
      'level',
      'category',
      'summary',
      'details',
      'timing',
      'compact',
    ]);
    if (
      !text(event.id, 36) ||
      !/^[a-f\d-]{36}$/i.test(event.id) ||
      !text(event.date, 32) ||
      !Number.isFinite(Date.parse(event.date)) ||
      !['info', 'warning', 'error'].includes(event.level as string) ||
      !text(event.category, 64) ||
      !text(event.summary, 4096) ||
      !(event.details === null || text(event.details, 8192)) ||
      !(event.timing === null || text(event.timing, 1024)) ||
      !(event.compact === undefined || text(event.compact, 160))
    )
      throw new Error('invalid_diagnostics');
  }
  if (new Set(rows.map((event) => event.id)).size !== rows.length)
    throw new Error('invalid_diagnostics');
  return item as unknown as DiagnosticsSnapshot;
}
