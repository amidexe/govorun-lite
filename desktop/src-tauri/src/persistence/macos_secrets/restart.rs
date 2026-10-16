//! Re-login uses the same current file store.
use super::*;
impl<D: Domain, K: Keychain> Owner<D, K> {
    pub(super) fn restart(&self, service: &str, state: &mut Cache) -> Result<Value, Error> {
        let target = self.keychain.restart(service)?;
        self.keychain.write(&target, AUTH, "null", false, false)?;
        let changes: Changes = [
            (MARKER.into(), Some(Cell::Scalar(json!(false)))),
            (PROFILE.into(), None),
        ].into();
        self.changed(changes)?;
        self.advance_boundary(false);
        *state = Cache::default();
        Ok(json!({"status":"available"}))
    }
}
