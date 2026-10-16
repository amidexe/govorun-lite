import { getContext } from 'svelte';
import type { DesktopApi } from '../../ipc/port';

export const desktopApiKey = Symbol('desktopApi');

/** The root supplies the native adapter; features may supply a fake explicitly. */
export function useDesktopApi(): DesktopApi {
  const api = getContext<DesktopApi>(desktopApiKey);
  if (!api) throw new Error('Desktop API is missing from the application context');
  return api;
}
