import { accountSnapshot, type AccountSnapshot } from './model.ts';
import type { AccountSnapshot as HostAccountSnapshot } from '../../ipc/generated/account';

const actionFailure = 'Не удалось выполнить действие. Проверьте связь и попробуйте ещё раз.';
const messageCodes = new Set(['account_device_limit', 'account_sign_in_failed', 'account_signed_out',
  'account_session_changed', 'account_storage_unavailable', 'account_storage_invalid',
  'account_invalid_response', 'account_invalid_request', 'account_network_unavailable',
  'account_response_too_large', 'account_terms_mismatch', 'account_consent_unconfirmed', 'account_http_error']);
const storageCopy: Record<string, { title: string; detail: string; action: string }> = {
  available: { title: 'Доступ к данным Говоруна восстановлен', detail: 'Доступ к сохранённым данным открыт.', action: 'Повторить проверку' },
  permissionRequired: { title: 'Нужно восстановить доступ', detail: 'Нажмите «Восстановить доступ» и разрешите Говоруну использовать сохранённый вход и лимит в системном окне. Обычная диктовка продолжает работать.', action: 'Восстановить доступ' },
  temporarilyUnavailable: { title: 'Не удалось открыть сохранённые данные Говоруна', detail: 'Диктовка работает без стилизации. Повторите проверку доступа; сохранённые данные остаются на месте.', action: 'Повторить проверку' },
  invalidData: { title: 'Не удалось прочитать сохранённые данные стилизации', detail: 'Диктовка работает без стилизации. Сохранённые данные не удалены. Обратитесь в поддержку для восстановления.', action: 'Повторить проверку' },
};

/** A host code becomes product copy here, without accepting arbitrary server text. */
export function accountHostSnapshot(value: unknown): AccountSnapshot {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('invalid_account');
  const host = value as HostAccountSnapshot;
  if (!host.storage || typeof host.storage.state !== 'string' || !Object.hasOwn(storageCopy, host.storage.state) ||
      (host.messageCode !== null && (typeof host.messageCode !== 'string' || !messageCodes.has(host.messageCode))) ||
      (host.providerNoticeCode !== null && host.providerNoticeCode !== 'account_provider_unavailable')) throw new Error('invalid_account');
  const message = host.messageCode === 'account_device_limit'
    ? `Подключено ${host.deviceLimit} устройства. Выберите, какое заменить.`
    : host.messageCode === 'account_sign_in_failed' ? 'Не удалось завершить вход. Проверьте связь и попробуйте ещё раз.'
    : host.messageCode === null ? null : actionFailure;
  return accountSnapshot({ scope: host.scope, status: host.status, displayName: host.displayName,
    message, busy: host.busy, syncMode: host.syncMode, providers: host.providers,
    providerNotice: host.providerNoticeCode === null ? null : 'Вход через выбранный сервис временно недоступен.',
    devices: host.devices, deviceLimit: host.deviceLimit, replacingDevice: host.replacingDevice,
    storage: { available: host.storage.available, state: host.storage.state, ...storageCopy[host.storage.state] } });
}
