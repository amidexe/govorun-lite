import { useDesktopApi } from '../../shared/state/desktop-context';
import { actionPayload, type ActionArguments, type SelfHostActions } from '../../ipc/actions';
import { onMount } from 'svelte';
import { selfHostSnapshot, type SelfHostSnapshot } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';

export function useSelfHost(api = useDesktopApi()) {
  const saved = useNativeSnapshot('selfHost.snapshot', selfHostSnapshot, 'selfhost.changed', api);
  let pending = $state(false),
    error = $state(''),
    disposed = false;
  let active: Promise<unknown> | null = null;
  async function cancelOperation(value: SelfHostSnapshot | null) {
    if (!value?.operationID) return;
    try {
      await api.invoke('selfHost.action', {
        action: 'cancel',
        operationID: value.operationID,
      });
    } catch {
      if (!disposed) error = 'Не удалось отменить проверку. Попробуйте ещё раз.';
    }
  }
  const cancel = () => {
    void cancelOperation(saved.value);
    void active?.then((value) => cancelOperation(selfHostSnapshot(value))).catch(() => {});
  };
  onMount(() => () => {
    disposed = true;
    cancel();
  });
  async function actForScope(scope: string | undefined, ...args: ActionArguments<SelfHostActions>) {
    if (pending || saved.failed || !scope) return null;
    pending = true;
    error = '';
    const request = api.invoke('selfHost.action', {
      ...actionPayload<SelfHostActions>(args),
      scope,
    });
    active = request;
    try {
      const value = selfHostSnapshot(await request);
      if (!disposed) saved.accept(value);
      else void cancelOperation(value);
      return value;
    } catch {
      if (!disposed) {
        error = 'Не удалось применить настройки. Проверьте поля и попробуйте ещё раз.';
        await saved.refresh();
      }
      return null;
    } finally {
      if (active === request) active = null;
      pending = false;
    }
  }
  function act(...args: ActionArguments<SelfHostActions>) {
    return actForScope(saved.value?.scope, ...args);
  }
  return {
    saved,
    act,
    actForScope,
    cancel,
    get pending() {
      return pending;
    },
    get error() {
      return error;
    },
    get busy() {
      return pending || saved.failed || saved.value?.busy === true;
    },
  };
}
