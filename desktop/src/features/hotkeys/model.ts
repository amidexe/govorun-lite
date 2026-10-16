import type { HotkeySnapshot } from '../../ipc/generated/hotkeys';
export type { HotkeySnapshot } from '../../ipc/generated/hotkeys';
import { stringEnum } from '../../shared/validation/string-enum.ts';

export function hotkeySnapshot(value: unknown): HotkeySnapshot {
  if (!value || typeof value !== 'object') throw new Error('invalid_hotkeys');
  const item = value as Record<string, unknown>;
  const tokens = (value: unknown) =>
    Array.isArray(value) &&
    value.length <= 8 &&
    value.every((token) => typeof token === 'string' && token.length > 0 && token.length <= 100);
  if (
    !tokens(item.dictation) ||
    !tokens(item.plain) ||
    !tokens(item.cancel) ||
    !tokens(item.preview) ||
    !(item.inputMode === undefined || stringEnum(item.inputMode, ['adaptive', 'hold'])) ||
    typeof item.canCapture !== 'boolean' ||
    !(
      item.captureID === null ||
      (typeof item.captureID === 'string' && item.captureID.length <= 64)
    ) ||
    ![null, 'dictation', 'plain', 'cancel'].includes(item.target as null | string) ||
    (item.captureID === null) !== (item.target === null) ||
    !(item.message === null || (typeof item.message === 'string' && item.message.length <= 300))
  )
    throw new Error('invalid_hotkeys');
  return item as unknown as HotkeySnapshot;
}
