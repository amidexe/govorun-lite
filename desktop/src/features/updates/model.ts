import type { UpdateSnapshot, UpdatesChannel as UpdateChannel } from '../../ipc/generated/updates';
export type { UpdateSnapshot, UpdatesChannel as UpdateChannel } from '../../ipc/generated/updates';
export type UpdateStatus = UpdateSnapshot['status'];
export function updateSnapshot(value: unknown): UpdateSnapshot {
  if (!value || typeof value !== 'object') throw new Error('invalid_update_snapshot');
  const item = value as UpdateSnapshot;
  if (
    ![
      'idle',
      'checking',
      'available',
      'current',
      'unavailable',
      'disabled',
      'downloading',
      'ready',
      'installing',
    ].includes(item.status) ||
    typeof item.dismissed !== 'boolean'
  )
    throw new Error('invalid_update_snapshot');
  if (
    item.release !== null &&
    (!item.release ||
      typeof item.release.version !== 'string' ||
      item.release.version.length > 64 ||
      !/^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(item.release.version) ||
      typeof item.release.notes !== 'string' ||
      [...item.release.notes].length > 1500)
  )
    throw new Error('invalid_update_snapshot');
  if (['available', 'downloading', 'ready', 'installing'].includes(item.status) && !item.release)
    throw new Error('invalid_update_snapshot');
  if (
    item.transfer != null &&
    (!Number.isSafeInteger(item.transfer.downloaded) ||
      !Number.isSafeInteger(item.transfer.total) ||
      item.transfer.downloaded < 0 ||
      item.transfer.total <= 0 ||
      item.transfer.downloaded > item.transfer.total ||
      item.transfer.total > 8_000_000_000)
  )
    throw new Error('invalid_update_snapshot');
  if (item.error != null && (typeof item.error !== 'string' || !/^[a-z_]{1,80}$/.test(item.error)))
    throw new Error('invalid_update_snapshot');
  if (item.channel !== undefined && !['stable', 'test'].includes(item.channel))
    throw new Error('invalid_update_snapshot');
  if (item.channelSelectable !== undefined && typeof item.channelSelectable !== 'boolean')
    throw new Error('invalid_update_snapshot');
  return {
    ...item,
    channel: item.channel ?? 'test',
    channelSelectable: item.channelSelectable ?? false,
  };
}

export function updateMessage(status: UpdateStatus): string {
  switch (status) {
    case 'checking':
      return 'Проверяем обновления…';
    case 'current':
      return 'В выбранном канале пока нет более новой версии.';
    case 'unavailable':
      return 'Не удалось проверить обновления. Попробуйте позже.';
    case 'disabled':
      return 'Проверка обновлений пока недоступна.';
    case 'available':
      return 'Доступна новая версия';
    case 'downloading':
      return 'Загружаем обновление…';
    case 'ready':
      return 'Обновление загружено и проверено.';
    case 'installing':
      return 'Устанавливаем обновление…';
    default:
      return 'Проверяйте новые версии Говоруна здесь.';
  }
}

export function updateError(code: string | null | undefined): string {
  if (!code) return '';
  if (code === 'updates_cancelled') return 'Загрузка отменена.';
  if (code === 'maintenance_busy')
    return 'Обновление готово. Завершите диктовку и попробуйте ещё раз.';
  if (
    ['updates_invalid_signature', 'updates_package_mismatch', 'updates_stale_release'].includes(
      code,
    )
  )
    return 'Не удалось проверить обновление. Попробуйте загрузить его ещё раз.';
  if (code === 'updates_download_failed' || code === 'updates_unavailable')
    return 'Не удалось загрузить обновление. Проверьте соединение и попробуйте ещё раз.';
  if (code === 'updates_recovery_required')
    return 'Не удалось завершить обновление. Нужна повторная установка приложения.';
  if (code === 'updates_install_unavailable')
    return 'Не удалось подготовить установку. Проверьте свободное место и доступ к папке приложения.';
  return 'Не удалось обновить приложение. Попробуйте ещё раз.';
}

export function downloadProgress(transfer: NonNullable<UpdateSnapshot['transfer']>): string {
  const megabytes = (bytes: number) => (bytes / (1024 * 1024)).toFixed(1).replace('.', ',');
  return `${megabytes(transfer.downloaded)} из ${megabytes(transfer.total)} МБ`;
}
