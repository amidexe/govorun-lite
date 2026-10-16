import type { AboutSnapshot } from '../../ipc/generated/about';
export type { AboutSnapshot, ApplicationLicense } from '../../ipc/generated/about';
export function aboutSnapshot(value: unknown): AboutSnapshot {
  if (!value || typeof value !== 'object') throw new Error('invalid_about');
  const item = value as AboutSnapshot;
  const text = (value: unknown) => typeof value === 'string' && value.length <= 1024;
  if (
    ![item.version, item.nativeVersion, item.platformBuild, item.buildID, item.nativeBuildID].every(text) ||
    !['keychain', 'dpapi'].includes(item.keyStorage) ||
    !Array.isArray(item.licenses) ||
    item.licenses.length > 100 ||
    !item.licenses.every((row) => row && [row.id, row.name, row.spdx, row.copyright].every(text)) ||
    new Set(item.licenses.map((row) => row.id)).size !== item.licenses.length
  )
    throw new Error('invalid_about');
  return item;
}
