import { onMount } from 'svelte';
import { snapshotRequests } from '../application/snapshot-requests';
import { distinctSnapshots } from '../application/distinct-snapshots';
import type { SnapshotCommand } from '../../ipc/commands';
import { responses } from '../../ipc/responses';
import { useDesktopApi } from './desktop-context';

export function useNativeSnapshot<T>(
  method: SnapshotCommand,
  parse: (value: unknown) => T,
  changedEvent: string,
  api = useDesktopApi(),
  options: { nativeRequired?: boolean; retryOnFailure?: boolean } = {},
) {
  let value = $state<T | null>(null),
    loading = $state(true),
    failed = $state(false);
  let refreshing = $state(false);
  let mounted = false, connected = true;
  let retry: ReturnType<typeof setTimeout> | null = null;
  function clearRetry() {
    if (retry !== null) clearTimeout(retry);
    retry = null;
  }
  const publish = distinctSnapshots<T>((result) => { value = result; });
  const requests = snapshotRequests(
    async () => parse(await api.invoke(method)),
    (result) => {
      publish(result);
      failed = false;
      clearRetry();
    },
    () => {
      failed = true;
      if (mounted && connected && options.retryOnFailure) {
        clearRetry();
        retry = setTimeout(() => { retry = null; void refresh(); }, 1000);
      }
    },
    () => {
      loading = false;
      refreshing = false;
    },
  );
  function refresh() {
    clearRetry();
    refreshing = true;
    return requests.refresh();
  }
  onMount(() => {
    mounted = true;
    void refresh();
    const focus = () => {
      void refresh();
    };
    window.addEventListener('focus', focus);
    const unsubscribe = api.subscribe((event) => {
      if (event.event === changedEvent) {
        // Feature owners publish complete snapshots. Use that observation instead
        // of starting another read that can race the next background update.
        try { requests.accept(parse(responses[method](event.data))); }
        catch { void refresh(); } // Older invalidation-only events still reconcile.
      }
      if (event.event === 'runtime.connected') { connected = true; void refresh(); }
      if (event.event === 'runtime.unavailable') {
        connected = false;
        clearRetry();
        if (options.nativeRequired === false) void refresh();
        else requests.unavailable();
      }
    });
    return () => {
      mounted = false;
      clearRetry();
      requests.dispose();
      unsubscribe();
      window.removeEventListener('focus', focus);
    };
  });
  return {
    get value() {
      return value;
    },
    get loading() {
      return loading;
    },
    get failed() {
      return failed;
    },
    get refreshing() {
      return refreshing;
    },
    refresh,
    accept(result: unknown) {
      requests.accept(parse(result));
    },
  };
}
