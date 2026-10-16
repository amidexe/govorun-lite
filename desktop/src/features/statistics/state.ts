import { statisticsSnapshot, type StatisticsSnapshot } from './model.ts';

/** Renderer confirmation lifecycle; persistence stays behind statistics.reset. */
export function statisticsResetController(ports: {
  scope: () => string | null;
  available: () => boolean;
  reset: () => Promise<unknown>;
  accept: (snapshot: StatisticsSnapshot) => void;
  changed: () => void;
}) {
  let confirmation: string | null = null;
  let resultScope: string | null = null;
  let busy = false,
    failed = false,
    completed = false,
    disposed = false;
  const allowed = () => !disposed && ports.available() && !!ports.scope();
  const notify = () => {
    if (!disposed) ports.changed();
  };
  function synchronize() {
    if (confirmation && (!allowed() || confirmation !== ports.scope())) {
      confirmation = null;
      notify();
    }
  }
  return {
    synchronize,
    get view() {
      return {
        enabled: allowed() && !busy,
        confirming: allowed() && confirmation === ports.scope() && confirmation !== null,
        busy,
        failed: resultScope === ports.scope() && failed,
        completed: resultScope === ports.scope() && completed,
      };
    },
    request() {
      if (!allowed() || busy) return;
      confirmation = ports.scope();
      resultScope = null;
      failed = false;
      completed = false;
      notify();
    },
    cancel() {
      confirmation = null;
      notify();
    },
    async confirm() {
      const scope = confirmation;
      confirmation = null;
      if (!scope || scope !== ports.scope() || !allowed() || busy) {
        notify();
        return;
      }
      busy = true;
      failed = false;
      completed = false;
      resultScope = scope;
      notify();
      try {
        const snapshot = statisticsSnapshot(await ports.reset());
        if (!disposed && scope === ports.scope() && allowed()) {
          ports.accept(snapshot);
          completed = true;
        }
      } catch {
        if (!disposed && scope === ports.scope()) failed = true;
      } finally {
        busy = false;
        notify();
      }
    },
    dispose() {
      disposed = true;
      confirmation = null;
    },
  };
}
