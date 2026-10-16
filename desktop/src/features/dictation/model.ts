import type { DictationSnapshot } from '../../ipc/generated/dictation';
export type { DictationSnapshot } from '../../ipc/generated/dictation';
import { stringEnum } from '../../shared/validation/string-enum.ts';

export type DictationField =
  | 'microphoneId'
  | 'playSounds'
  | 'muteAudio'
  | 'maxMinutes'
  | 'acceleration'
  | 'voiceMarker'
  | 'launchAtLogin'
  | 'pushToTalk'
  | 'keepModelLoaded'
  | 'alwaysOnMicrophone'
  | 'inputChannel';

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_dictation');
  return value as Record<string, unknown>;
}
function text(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 1024;
}

export function dictationSnapshot(value: unknown): DictationSnapshot {
  const item = record(value),
    permissions = record(item.permissions);
  if (
    !Array.isArray(item.microphones) ||
    item.microphones.length > 256 ||
    !item.microphones.every((value) => {
      const device = record(value);
      return text(device.id) && text(device.name);
    }) ||
    new Set(item.microphones.map((value) => value.id)).size !== item.microphones.length ||
    !(item.microphoneId === null || text(item.microphoneId)) ||
    !stringEnum(item.launchAtLogin, ['enabled', 'disabled', 'approvalRequired', 'unavailable']) ||
    typeof item.playSounds !== 'boolean' ||
    typeof item.muteAudio !== 'boolean' ||
    !Number.isSafeInteger(item.maxMinutes) ||
    Number(item.maxMinutes) < 0 ||
    Number(item.maxMinutes) > 180 ||
    !stringEnum(item.durationMode, ['local', 'styles', 'unlimited']) ||
    !stringEnum(item.acceleration, ['automatic', 'cpu', 'gpu']) ||
    !stringEnum(item.voiceMarker, ['off', 'microphone', 'text']) ||
    !Number.isSafeInteger(item.markerMinimum) ||
    Number(item.markerMinimum) < 0 ||
    !stringEnum(permissions.microphone, [
      'authorized',
      'notDetermined',
      'denied',
      'restricted',
      'unknown',
    ]) ||
    typeof permissions.accessibility !== 'boolean'
  )
    throw new Error('invalid_dictation');
  if (
    ['pushToTalk', 'keepModelLoaded', 'inputChannel', 'inputChannels'].some((key) => key in item) &&
    (typeof item.pushToTalk !== 'boolean' ||
      typeof item.keepModelLoaded !== 'boolean' ||
      !Number.isSafeInteger(item.inputChannels) ||
      Number(item.inputChannels) < 1 ||
      Number(item.inputChannels) > 32 ||
      !(
        item.inputChannel === null ||
        (Number.isSafeInteger(item.inputChannel) &&
          Number(item.inputChannel) >= 0 &&
          Number(item.inputChannel) < 32)
      ))
  )
    throw new Error('invalid_dictation');
  if ('alwaysOnMicrophone' in item && typeof item.alwaysOnMicrophone !== 'boolean')
    throw new Error('invalid_dictation');
  return item as unknown as DictationSnapshot;
}
