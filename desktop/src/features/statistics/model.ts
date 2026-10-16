import { stringEnum } from '../../shared/validation/string-enum.ts';

import type { StatisticsSnapshot } from '../../ipc/generated/statistics';
export type { StatisticsSnapshot, StatisticsDay as DayUsage } from '../../ipc/generated/statistics';
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_snapshot');
  return value as Record<string, unknown>;
}
function count(value: unknown): boolean {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}
function usage(value: unknown): boolean {
  const item = record(value);
  return count(item.words) && count(item.seconds) && count(item.sessions);
}
const datePattern = /^\d{4}-\d{2}-\d{2}$/;
function validDate(value: unknown): boolean {
  if (typeof value !== 'string' || !datePattern.test(value)) return false;
  const parsed = new Date(`${value}T12:00:00Z`);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 10) === value;
}

export function statisticsSnapshot(value: unknown): StatisticsSnapshot {
  const item = record(value);
  if (
    !validDate(item.today) ||
    !usage(item.total) ||
    typeof item.signedIn !== 'boolean' ||
    typeof item.hasCloudSnapshot !== 'boolean' ||
    typeof item.storageRecovered !== 'boolean' ||
    typeof item.storageReadOnly !== 'boolean' ||
    !stringEnum(item.syncMode, ['evaluating', 'pending', 'enabled', 'localOnly']) ||
    !Array.isArray(item.days) ||
    item.days.length > 31 ||
    !item.days.every((day) => {
      const row = record(day);
      return usage(row) && validDate(row.date);
    }) ||
    new Set(item.days.map((day) => day.date)).size !== item.days.length
  )
    throw new Error('invalid_statistics');
  return item as unknown as StatisticsSnapshot;
}
