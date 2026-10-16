//! Deferred OS registration: first-run setup stays usable before permission.
//! The accepted bindings change only after an existing hook acknowledges them.
pub struct Registration<B, H> {
    pub bindings: B,
    pub hook: Option<H>,
    closed: bool,
}
impl<B: PartialEq, H> Registration<B, H> {
    pub fn new(bindings: B) -> Self {
        Self {
            bindings,
            hook: None,
            closed: false,
        }
    }
    pub fn configure(
        &mut self,
        next: B,
        permitted: bool,
        start: impl FnOnce(&B) -> Result<H, &'static str>,
        replace: impl FnOnce(&H, &B) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        if self.closed {
            return Err("dictation_stopped");
        }
        if !permitted {
            self.hook.take();
        } else if let Some(hook) = &self.hook {
            if self.bindings != next {
                replace(hook, &next)?;
            }
        } else {
            self.hook = match start(&next) {
                Ok(hook) => Some(hook),
                // macOS can grant accessibility before the event tap becomes
                // available. Keep first-run setup reachable in that interval.
                Err("hotkeys_permission_required") => None,
                Err(error) => return Err(error),
            };
        }
        self.bindings = next;
        Ok(())
    }
    pub fn shutdown(&mut self) {
        self.closed = true;
        self.hook.take();
    }
}
