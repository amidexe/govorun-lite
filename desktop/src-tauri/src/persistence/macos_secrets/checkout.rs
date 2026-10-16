//! Physical storage and CAS for pending checkout documents.
use super::*;
impl<D: Domain, K: Keychain> Owner<D, K> {
    pub(super) fn checkout(
        &self,
        service: &str,
        method: &str,
        key: &str,
        params: &Value,
    ) -> Result<Value, Error> {
        if !matches!(method, "storage.read" | "storage.write" | "storage.delete") {
            return Err(Error::Invalid);
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        let _guard = self.state.lock().map_err(|_| Error::Unavailable)?;
        if self.epoch.load(Ordering::SeqCst) != epoch { return Err(Error::Unavailable); }
        let raw = self.keychain.read(service, key, false, false)?;
        let current = raw
            .as_deref()
            .map(|s| parse(s, 65_536))
            .transpose()?
            .filter(|v| !v.is_null());
        let rev = current.as_ref().and(raw.as_deref()).map(revision);
        if method == "storage.read" {
            return Ok(match current {
                None => json!({"status":"missing"}),
                Some(value) => json!({"status":"available","revision":rev,"value":value}),
            });
        }
        if expected(params)? != rev.as_deref() {
            return Err(Error::Conflict);
        }
        let value = if method == "storage.delete" {
            Value::Null
        } else {
            params
                .get("value")
                .filter(|v| v.is_object())
                .cloned()
                .ok_or(Error::Invalid)?
        };
        let raw = value.to_string();
        parse(&raw, 65_536)?;
        self.keychain.write(service, key, &raw, false, false)?;
        Ok(if method == "storage.delete" {
            json!({"status":"available"})
        } else {
            json!({"status":"available","value":value,"revision":revision(&raw)})
        })
    }
}
