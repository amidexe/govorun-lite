import { useDesktopApi } from '../../shared/state/desktop-context';
import type { DictationAction, DictationFields, DictationSetting } from '../../ipc/actions';
import { dictationSnapshot } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';

export function useDictation(api = useDesktopApi()) {
  const settings = useNativeSnapshot(
    'dictation.snapshot',
    dictationSnapshot,
    'dictation.changed',
    api,
  );
  let pending = $state(false),
    error = $state<string | null>(null);
  async function act(params: DictationAction) {
    if (pending || settings.failed) return;
    pending = true;
    error = null;
    try {
      dictationSnapshot(await api.invoke('dictation.action', params));
      await settings.refresh();
    } catch {
      error = 'Не удалось применить настройку. Попробуйте ещё раз.';
      await settings.refresh();
    } finally {
      pending = false;
    }
  }
  return {
    settings,
    act,
    set<Field extends keyof DictationFields>(field: Field, value: DictationFields[Field]) {
      return act({ action: 'set', field, value } as DictationSetting);
    },
    get busy() {
      return pending || settings.failed;
    },
    get error() {
      return error;
    },
  };
}
