// Native change notifications can arrive in bursts. Read one snapshot at a time,
// then reconcile once more if something changed while the read was in flight.
export function snapshotRequests<T>(
  read: () => Promise<T>,
  publish: (value: T) => void,
  fail: () => void,
  settled: () => void,
) {
  let active: Promise<void> | null = null,
    requested = false,
    revision = 0,
    disposed = false;
  function refresh(): Promise<void> {
    if (disposed) return Promise.resolve();
    requested = true;
    if (active) return active;
    active = Promise.resolve()
      .then(async () => {
        do {
          requested = false;
          const expected = revision;
          try {
            const value = await read();
            if (!disposed && expected === revision) publish(value);
          } catch {
            if (!disposed && expected === revision) fail();
          }
        } while (requested && !disposed);
      })
      .finally(() => {
        active = null;
        if (!disposed) settled();
      });
    return active;
  }
  return {
    refresh,
    accept(value: T) {
      if (!disposed) {
        revision++;
        requested = false;
        publish(value);
        settled();
      }
    },
    unavailable() {
      revision++;
      requested = false;
      if (!disposed) {
        fail();
        settled();
      }
    },
    dispose() {
      disposed = true;
      revision++;
      requested = false;
    },
  };
}
