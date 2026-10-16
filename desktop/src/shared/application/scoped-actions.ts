/** Serializes deferred actions within the account scope that requested them. */
export function scopedActions<Request>(
  perform: (request: Request) => Promise<boolean>,
  currentScope: () => string | null | undefined,
  waitForActive: (request: Request) => boolean,
) {
  let active: Promise<boolean> | null = null;

  function submit(request: Request): Promise<boolean> {
    if (active) {
      if (!waitForActive(request)) return Promise.resolve(false);
      const scope = currentScope();
      const resume = () => (scope && scope === currentScope() ? submit(request) : false);
      return active.then(resume, resume);
    }

    const operation = perform(request);
    const completion = operation.finally(() => {
      if (active === completion) active = null;
    });
    active = completion;
    return completion;
  }

  return { submit };
}
