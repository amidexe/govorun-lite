import { stringEnum } from '../../shared/validation/string-enum.ts';

import type { AccountDevice, AccountSnapshot as WireAccountSnapshot } from '../../ipc/generated/account';
export type { AccountDevice } from '../../ipc/generated/account';

// Only translated presentation fields differ from the shared wire snapshot.
export interface AccountSnapshot extends Omit<WireAccountSnapshot,
  'status' | 'syncMode' | 'providers' | 'messageCode' | 'providerNoticeCode' | 'storage'> {
  status: 'signedOut' | 'signingIn' | 'signedIn' | 'failed';
  message: string | null;
  syncMode: 'evaluating' | 'pending' | 'enabled' | 'localOnly';
  providers: { id: 'yandex' | 'vk'; enabled: boolean }[];
  providerNotice: string | null;
  storage: { available: boolean; state: 'available' | 'permissionRequired' | 'temporarilyUnavailable' | 'invalidData'; title: string; detail: string; action: string };
}

export type AccountAction =
  | 'signIn'
  | 'cancelSignIn'
  | 'signOut'
  | 'refresh'
  | 'merge'
  | 'keepLocal'
  | 'requestSync'
  | 'recoverStorage'
  | 'restartStorage'
  | 'revokeDevice'
  | 'replaceDevice'
  | 'deleteAccount'
  | 'openLink';

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_account');
  return value as Record<string, unknown>;
}
function text(value: unknown, limit: number): value is string {
  return typeof value === 'string' && value.length <= limit;
}
function optionalText(value: unknown, limit: number): boolean {
  return value === null || text(value, limit);
}

export function accountSnapshot(value: unknown): AccountSnapshot {
  const item = record(value),
    storage = record(item.storage);
  if (
    !text(item.scope, 160) ||
    !item.scope ||
    !stringEnum(item.status, ['signedOut', 'signingIn', 'signedIn', 'failed']) ||
    !optionalText(item.displayName, 512) ||
    !optionalText(item.message, 2000) ||
    typeof item.busy !== 'boolean' ||
    !stringEnum(item.syncMode, ['evaluating', 'pending', 'enabled', 'localOnly']) ||
    !Array.isArray(item.providers) ||
    item.providers.length > 2 ||
    !item.providers.every((value) => {
      const provider = record(value);
      return stringEnum(provider.id, ['yandex', 'vk']) && typeof provider.enabled === 'boolean';
    }) ||
    new Set(item.providers.map((provider) => provider.id)).size !== item.providers.length ||
    !optionalText(item.providerNotice, 1000) ||
    !Array.isArray(item.devices) ||
    item.devices.length > 100 ||
    !item.devices.every((value) => {
      const device = record(value);
      return (
        text(device.id, 128) &&
        device.id.length > 0 &&
        text(device.name, 512) &&
        text(device.subtitle, 512) &&
        text(device.confirmation, 2000) &&
        typeof device.current === 'boolean'
      );
    }) ||
    new Set(item.devices.map((device) => device.id)).size !== item.devices.length ||
    !Number.isSafeInteger(item.deviceLimit) ||
    Number(item.deviceLimit) < 1 ||
    Number(item.deviceLimit) > 100 ||
    typeof item.replacingDevice !== 'boolean' ||
    typeof storage.available !== 'boolean' ||
    !stringEnum(storage.state, ['available', 'permissionRequired', 'temporarilyUnavailable', 'invalidData']) ||
    !text(storage.title, 512) ||
    !text(storage.detail, 2000) ||
    !text(storage.action, 128)
  ) {
    throw new Error('invalid_account');
  }
  return item as unknown as AccountSnapshot;
}
