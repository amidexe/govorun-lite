import type { SelfHostConfiguration, SelfHostSnapshot } from '../../ipc/generated/self-host';
export type { SelfHostConfiguration, SelfHostSnapshot } from '../../ipc/generated/self-host';
const record = (value: unknown): Record<string, unknown> => {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_self_host');
  return value as Record<string, unknown>;
};
const string = (value: unknown, max: number) => typeof value === 'string' && value.length <= max;
export function selfHostSnapshot(value: unknown): SelfHostSnapshot {
  const item = record(value),
    config = record(item.configuration);
  if (
    !string(item.scope, 100) ||
    !item.scope ||
    !(
      typeof config.provider === 'string' &&
      ['ollama', 'openai', 'gemini'].includes(config.provider)
    ) ||
    !string(config.endpoint, 8192) ||
    !string(config.model, 1000) ||
    !string(config.prompt, 262144) ||
    !string(config.proxyURL, 8192) ||
    typeof config.proxyEnabled !== 'boolean' ||
    Object.keys(config).some(
      (key) =>
        !['provider', 'endpoint', 'model', 'prompt', 'proxyEnabled', 'proxyURL'].includes(key),
    ) ||
    ![
      'routeEnabled',
      'stylingEnabled',
      'configured',
      'consent',
      'verified',
      'busy',
      'cancelling',
      'failed',
    ].every((key) => typeof item[key] === 'boolean') ||
    !(
      typeof item.keyState === 'string' &&
      ['missing', 'saved', 'inaccessible', 'staleReference'].includes(item.keyState)
    ) ||
    !(item.operationID === null || string(item.operationID, 64)) ||
    ![null, 'connect', 'removeKey', 'clear'].includes(item.operation as string | null) ||
    item.busy !== (item.operation !== null) ||
    !(item.message === null || string(item.message, 500)) ||
    item.busy !== (item.operationID !== null) ||
    Object.keys(item).some(
      (key) =>
        ![
          'scope',
          'configuration',
          'routeEnabled',
          'stylingEnabled',
          'configured',
          'consent',
          'verified',
          'keyState',
          'busy',
          'cancelling',
          'operationID',
          'operation',
          'message',
          'failed',
        ].includes(key),
    )
  )
    throw new Error('invalid_self_host');
  return item as unknown as SelfHostSnapshot;
}
export function canConnect(config: SelfHostConfiguration | null, consent: boolean) {
  return (
    !!config &&
    consent &&
    !!config.endpoint.trim() &&
    !!config.model.trim() &&
    config.prompt.trim().length >= 3 &&
    (!config.proxyEnabled || !!config.proxyURL.trim())
  );
}

export function selfHostStatus(value: SelfHostSnapshot) {
  if (!value.routeEnabled) return 'Свой сервер отключён';
  if (!value.configured || !value.consent)
    return 'Нужно настроить подключение и подтвердить передачу данных';
  return value.stylingEnabled
    ? 'Свой сервер подключён'
    : 'Свой сервер настроен · стилизация отключена';
}

export function configurationChanged(
  draft: SelfHostConfiguration | null,
  source: SelfHostConfiguration | null,
) {
  return !!draft && !!source && JSON.stringify(draft) !== JSON.stringify(source);
}
