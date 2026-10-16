import { useDesktopApi } from '../../shared/state/desktop-context';
import { actionPayload, type ActionArguments, type AccountActions } from '../../ipc/actions';
import { accountSnapshot } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';

export function useAccount(api = useDesktopApi()) {
  const account = useNativeSnapshot('account.snapshot', accountSnapshot, 'account.changed', api);
  let pending = $state(false),
    error = $state<string | null>(null);
  let errorScope = $state<string | null>(null);
  async function act(...args: ActionArguments<AccountActions>) {
    const request = actionPayload<AccountActions>(args);
    const { action } = request;
    const scope = account.value?.scope;
    if (!scope || pending || (account.failed && action !== 'recoverStorage')) return;
    pending = true;
    error = null;
    errorScope = scope;
    try {
      accountSnapshot(await api.invoke('account.action', { ...request, scope }));
      await account.refresh();
    } catch {
      if (scope === account.value?.scope && action !== 'recoverStorage') {
        error = 'Не удалось выполнить действие. Проверьте связь и попробуйте ещё раз.';
      }
      void account.refresh();
    } finally {
      pending = false;
    }
  }
  return {
    account,
    act,
    get pending() {
      return pending;
    },
    get error() {
      return errorScope === account.value?.scope ? error : null;
    },
  };
}
