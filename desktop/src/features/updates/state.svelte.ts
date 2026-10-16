import { getContext, onMount } from 'svelte';
import { updateSnapshot, updateError, type UpdateSnapshot, type UpdateChannel } from './model';

import type { DesktopApi } from '../../ipc/port';

export const updatesKey = Symbol('updates');
export function useUpdates() {
  const updates = getContext<ReturnType<typeof createUpdates>>(updatesKey);
  if (!updates) throw new Error('Updates are missing from the application context');
  onMount(() => updates.connect());
  return updates;
}

export function createUpdates(api: DesktopApi) {
  let snapshot = $state<UpdateSnapshot | null>(null),
    busy = $state(false),
    opening = $state(false),
    error = $state('');
  let inFlight: Promise<void> | null = null;
  let generation = 0;
  async function check(manual = false) {
    if (inFlight) return inFlight;
    if (opening) return;
    busy = true;
    error = '';
    generation++;
    inFlight = api
      .invoke('updates.check', { manual })
      .then((value) => {
        snapshot = updateSnapshot(value);
      })
      .catch(() => {
        if (manual) error = 'Не удалось проверить обновления. Попробуйте позже.';
      })
      .finally(() => {
        busy = false;
        inFlight = null;
      });
    return inFlight;
  }
  async function act(action: 'dismiss' | 'download' | 'install' | 'cancel') {
    if (action === 'cancel') {
      try {
        snapshot = updateSnapshot(await api.invoke('updates.cancel'));
      } catch {
        error = 'Не удалось отменить загрузку. Попробуйте ещё раз.';
      }
      return;
    }
    if (!snapshot?.release || opening || busy) return;
    opening = true;
    error = '';
    generation++;
    try {
      snapshot = updateSnapshot(
        await api.invoke(`updates.${action}`, { version: snapshot.release.version }),
      );
    } catch {
      error =
        action === 'dismiss'
          ? 'Не удалось сохранить выбор. Попробуйте ещё раз.'
          : 'Не удалось подготовить обновление. Попробуйте ещё раз.';
    } finally {
      opening = false;
    }
  }

  async function selectChannel(channel: UpdateChannel) {
    if (opening || busy || !snapshot?.channelSelectable || snapshot.channel === channel) return;
    opening = true;
    error = '';
    generation++;
    try {
      snapshot = updateSnapshot(await api.invoke('updates.channel', { channel }));
    } catch {
      error = 'Не удалось сменить канал. Завершите обновление и попробуйте ещё раз.';
    } finally {
      opening = false;
    }
    if (!error) await check(true);
  }

  function connect() {
    let disposed = false;
    const request = generation;
    void api
      .invoke('updates.snapshot')
      .then((value) => {
        if (!disposed && !busy && request === generation) snapshot = updateSnapshot(value);
      })
      .catch(() => {})
      .finally(() => {
        if (!disposed) void check();
      });
    const focus = () => {
      void check();
    };
    const visible = () => {
      if (document.visibilityState === 'visible') void check();
    };
    const unsubscribe = api.subscribe((event) => {
      if (event.event === 'updates.changed') {
        try {
          snapshot = updateSnapshot(event.data);
        } catch {
          error = 'Не удалось получить состояние обновления.';
        }
      }
    });
    window.addEventListener('focus', focus);
    document.addEventListener('visibilitychange', visible);
    return () => {
      disposed = true;
      unsubscribe();
      window.removeEventListener('focus', focus);
      document.removeEventListener('visibilitychange', visible);
    };
  }
  return {
    connect,
    get snapshot() {
      return snapshot;
    },
    get busy() {
      return busy;
    },
    get opening() {
      return opening;
    },
    get error() {
      return error || updateError(snapshot?.error);
    },
    check,
    act,
    selectChannel,
  };
}
