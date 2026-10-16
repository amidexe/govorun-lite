import { useDesktopApi } from '../../shared/state/desktop-context';
import {
  actionPayload,
  type ActionArguments,
  type ActionPayload,
  type BillingActions,
} from '../../ipc/actions';
import { billingSnapshot } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
import { scopedActions } from '../../shared/application/scoped-actions';

export function useBilling(api = useDesktopApi()) {
  const billing = useNativeSnapshot('billing.snapshot', billingSnapshot, 'billing.changed', api);
  let pending = $state(false),
    error = $state<string | null>(null);
  let errorScope = $state<string | null>(null);
  let requested = $state(false);
  $effect(() => {
    if (!pending && (!billing.value?.busy || billing.value.scope !== errorScope)) requested = false;
  });
  async function perform(request: ActionPayload<BillingActions>): Promise<boolean> {
    const { action } = request;
    const scope = billing.value?.scope;
    if (
      !scope ||
      pending ||
      billing.failed ||
      (billing.value?.busy &&
        !['dismissPreview', 'dismissPromo', 'resumeCheckout'].includes(action))
    )
      return false;
    pending = true;
    requested = true;
    error = null;
    errorScope = scope;
    try {
      const result = billingSnapshot(await api.invoke('billing.action', { ...request, scope }));
      if (scope === billing.value?.scope && result.scope === scope) billing.accept(result);
      else await billing.refresh();
      return scope === billing.value?.scope && !billing.failed;
    } catch {
      if (scope === billing.value?.scope)
        error = 'Не удалось выполнить действие. Попробуйте ещё раз.';
      await billing.refresh();
      return false;
    } finally {
      pending = false;
    }
  }
  const actions = scopedActions(
    perform,
    () => billing.value?.scope,
    ({ action }) => action === 'dismissPreview' || action === 'dismissPromo',
  );
  function act(...args: ActionArguments<BillingActions>): Promise<boolean> {
    return actions.submit(actionPayload<BillingActions>(args));
  }
  async function retry() {
    error = null;
    if (billing.failed || !billing.value) await billing.refresh();
    return act('refresh');
  }
  return {
    billing,
    act,
    retry,
    get error() {
      return errorScope === billing.value?.scope ? error : null;
    },
    get busy() {
      return (
        pending ||
        billing.failed ||
        billing.value?.busy === true ||
        billing.value?.status.loading === true
      );
    },
    get refreshingByUser() {
      return pending || (requested && billing.value?.busy === true);
    },
  };
}
