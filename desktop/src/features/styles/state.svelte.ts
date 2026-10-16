import { useDesktopApi } from '../../shared/state/desktop-context';
import {
  actionPayload,
  type ActionArguments,
  type ActionPayload,
  type StylesActions,
} from '../../ipc/actions';
import { stylesSnapshot } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
import { scopedActions } from '../../shared/application/scoped-actions';

export function useStyles(api = useDesktopApi()) {
  const styles = useNativeSnapshot('styles.snapshot', stylesSnapshot, 'styles.changed', api,
    { retryOnFailure: true });
  let pending = $state(false),
    error = $state<string | null>(null);
  let errorScope = $state<string | null>(null);
  async function perform(request: ActionPayload<StylesActions>): Promise<boolean> {
    const { action } = request;
    const scope = styles.value?.scope;
    if (!scope || pending || styles.failed) return false;
    pending = true;
    error = null;
    errorScope = scope;
    try {
      stylesSnapshot(await api.invoke('styles.action', { ...request, scope }));
      await styles.refresh();
      return scope === styles.value?.scope && !styles.failed;
    } catch {
      if (scope === styles.value?.scope)
        error = 'Не удалось применить изменения. Попробуйте ещё раз.';
      await styles.refresh();
      return false;
    } finally {
      pending = false;
    }
  }
  const actions = scopedActions(
    perform,
    () => styles.value?.scope,
    ({ action }) => action === 'cancel',
  );
  function act(...args: ActionArguments<StylesActions>): Promise<boolean> {
    return actions.submit(actionPayload<StylesActions>(args));
  }
  return {
    styles,
    act,
    get pending() {
      return pending;
    },
    get error() {
      return errorScope === styles.value?.scope ? error : null;
    },
    get busy() {
      return pending || styles.failed || styles.value?.busy === true;
    },
  };
}
