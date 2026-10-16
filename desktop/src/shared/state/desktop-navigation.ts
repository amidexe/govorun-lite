import { getContext, setContext } from 'svelte';
import type { DesktopApi } from '../../ipc/port';

type SettingsRoute = NonNullable<Parameters<DesktopApi['window']>[1]>;
export type DesktopRoute = 'home' | 'settings' | SettingsRoute;
type Navigate = (route: DesktopRoute) => void;
const navigationKey = Symbol('desktopNavigation');

/** In-window links keep the current native window, position and focus. */
export function provideDesktopNavigation(navigate: Navigate): void {
  setContext(navigationKey, navigate);
}

export function useDesktopNavigation(): Navigate {
  const navigate = getContext<Navigate>(navigationKey);
  if (!navigate) throw new Error('Desktop navigation is missing from the application context');
  return navigate;
}
