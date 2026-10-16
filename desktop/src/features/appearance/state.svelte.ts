import { appearanceSnapshot, type ThemeMode } from './model';
export type { ThemeMode } from './model';
import { getContext } from 'svelte';
import type { DesktopApi } from '../../ipc/port';

export const appearanceKey = Symbol('appearance');
export function useAppearance(): ReturnType<typeof createAppearance> {
  const appearance = getContext<ReturnType<typeof createAppearance>>(appearanceKey);
  if (!appearance) throw new Error('Appearance is missing from the application context');
  return appearance;
}

export function createAppearance(api: DesktopApi) {
  let mode = $state<ThemeMode>('system'),
    pending = $state(false),
    error = $state<string | null>(null);
  let revision = -1;

  function accept(value: unknown) {
    const snapshot = appearanceSnapshot(value);
    if (snapshot.revision < revision) return;
    revision = snapshot.revision;
    mode = snapshot.mode;
    document.documentElement.dataset.theme = mode;
  }

  async function initialize() {
    api.subscribe((event) => {
      if (event.event === 'appearance.changed') {
        try {
          accept(event.data);
          error = null;
        } catch {
          error = 'Не удалось обновить тему.';
        }
      }
    });
    try {
      accept(await api.invoke('appearance.snapshot'));
    } catch {
      error = 'Не удалось загрузить тему.';
    }
  }

  return {
    initialize,
    get mode() {
      return mode;
    },
    get pending() {
      return pending;
    },
    get error() {
      return error;
    },
    async set(next: ThemeMode) {
      if (pending) return;
      pending = true;
      error = null;
      try {
        accept(await api.invoke('appearance.set', { mode: next }));
      } catch {
        error = 'Не удалось сохранить тему. Попробуйте ещё раз.';
      } finally {
        pending = false;
      }
    },
  };
}
