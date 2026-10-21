import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { desktopMethods, type DesktopMethod } from './ipc/registry.ts';
import type { DesktopApi, NativeEvent } from './ipc/port';
import { desktopClient } from './ipc/client';
import type { RecordingIndicatorApi } from './features/dictation/indicator';

export async function installDesktopApi(): Promise<DesktopApi> {
  const listeners = new Set<(event: NativeEvent) => void>();
  await listen<NativeEvent>('govorun:event', (event) => {
    for (const listener of listeners) listener(event.payload);
  });
  const api: DesktopApi = {
    ...desktopClient(async (method, request) => {
      const params = request as Record<string, unknown>;
      if (method === 'window.layout') return invoke('window_layout', { home: params.home });
      if (method === 'runtime.reconnect') return invoke('native_reconnect');
      if (method === 'updates.snapshot') return invoke('updates');
      if (method === 'updates.check')
        return invoke('updates', { action: 'check', manual: params.manual === true });
      if (method === 'updates.channel')
        return invoke('updates', { action: 'channel', channel: params.channel });
      if (
        ['updates.dismiss', 'updates.download', 'updates.install', 'updates.cancel'].includes(
          method,
        )
      )
        return invoke('updates', {
          action: method.slice(8),
          version: params.version,
        });
      if (method === 'appearance.snapshot') return invoke('appearance');
      if (method === 'appearance.set') return invoke('appearance', { mode: params.mode });
      if (method === 'setup.snapshot') return invoke('setup');
      if (method === 'setup.save') return invoke('setup', { change: params });
      if (method === 'setup.reset' && params.confirmed === true)
        return invoke('setup', { reset: true });
      if (method === 'data.erase' && params.confirmed === true)
        return invoke('erase_local_data', { confirmed: true });
      if (!desktopMethods.includes(method as DesktopMethod)) {
        throw new Error('unsupported_command');
      }
      return invoke('native_command', { method, params });
    }),
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    window(action, route) {
      void invoke('window_action', { action, route }).catch(() => {
        for (const listener of listeners) listener({ event: 'window.unavailable', data: null });
      });
    },
  };
  Object.defineProperty(window, 'govorun', {
    value: Object.freeze(api),
    writable: false,
    configurable: false,
  });
  return api;
}

export function recordingIndicatorApi(): RecordingIndicatorApi {
  return {
    async subscribe(listener) {
      return listen<unknown>('recording:state', ({ payload }) => listener(payload));
    },
    snapshot: () => invoke('recording_indicator', { action: 'snapshot' }),
    cancel: () => invoke('recording_indicator', { action: 'cancel' }),
  };
}
