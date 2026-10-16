import { useDesktopApi } from '../../shared/state/desktop-context';
import { setupSnapshot, type SetupProgress } from './model';
import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';

export function useSetup(api = useDesktopApi()) {
  const saved = useNativeSnapshot('setup.snapshot', setupSnapshot, 'setup.changed', api);
  let pending = $state(false),
    error = $state('');
  async function save(progress: SetupProgress) {
    if (pending || !saved.value || saved.failed) return false;
    pending = true;
    error = '';
    try {
      saved.accept(
        await api.invoke('setup.save', {
          expectedRevision: saved.value.revision,
          progress,
        }),
      );
      return true;
    } catch {
      error = 'Не удалось сохранить этот шаг. Попробуйте ещё раз.';
      await saved.refresh();
      return false;
    } finally {
      pending = false;
    }
  }
  async function reset() {
    if (pending) return;
    pending = true;
    error = '';
    try {
      saved.accept(await api.invoke('setup.reset', { confirmed: true }));
    } catch {
      error = 'Не удалось начать настройку заново.';
    } finally {
      pending = false;
    }
  }
  return {
    saved,
    save,
    reset,
    get pending() {
      return pending;
    },
    get error() {
      return error;
    },
  };
}
