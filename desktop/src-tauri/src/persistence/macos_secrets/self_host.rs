//! Physical storage of existing self-hosted keys; readers are retained.
use super::*;
impl<D: Domain, K: Keychain> Owner<D, K> {
    pub(super) fn self_host(&self, method: &str, key: &str, params: &Value) -> Result<Value, Error> {
        let provider = key.strip_prefix("selfHost.").ok_or(Error::Invalid)?;
        if !matches!(provider, "ollama" | "openai" | "gemini") {
            return Err(Error::Invalid);
        }
        let service = super::super::macos_profile::llm_service(&self.identity);
        let account = format!("llmApiKey_{provider}");
        let hint = format!("llmApiKeySaved_{provider}");
        match method {
            "secret.read" => {
                let (state, key) = match self.keychain.read(
                    &service,
                    &account,
                    params["allowInteraction"] == true,
                    true,
                ) {
                    Ok(Some(raw)) => (if raw.is_empty() { "missing" } else { "saved" }, Some(raw)),
                    Ok(None) => {
                        let values = self.domain.read().map_err(|_| Error::Unavailable)?;
                        let legacy = string_cell(values.get(&account)).or_else(|| {
                            if provider == "gemini" {
                                string_cell(values.get("llmApiKey"))
                            } else {
                                None
                            }
                        });
                        if let Some(raw) = legacy.filter(|s| !s.is_empty()) {
                            ("saved", Some(raw))
                        } else {
                            (
                                if self.domain.boolean(&hint) {
                                    "staleReference"
                                } else {
                                    "missing"
                                },
                                None,
                            )
                        }
                    }
                    Err(_) => ("inaccessible", None),
                };
                Ok(json!({"status":"available","value":{"state":state,"key":key}}))
            }
            "secret.write" if exact(&params["value"], &["key"]) => {
                let raw = params["value"]["key"]
                    .as_str()
                    .filter(|s| s.len() <= 16384)
                    .ok_or(Error::Invalid)?;
                self.keychain.write(&service, &account, raw, false, true)?;
                self.changed([(hint, Some(Cell::Scalar(json!(true))))].into())?;
                Ok(json!({"status":"available"}))
            }
            "secret.delete" => {
                let services = super::super::macos_profile::llm_cleanup_services(&self.identity);
                self.keychain.delete(
                    &services.iter().map(String::as_str).collect::<Vec<_>>(),
                    &account,
                    true,
                    true,
                )?;
                let mut changes: Changes =
                    [(hint, Some(Cell::Scalar(json!(false)))), (account, None)].into();
                if provider == "gemini" {
                    changes.insert("llmApiKey".into(), None);
                }
                self.changed(changes)?;
                Ok(json!({"status":"available"}))
            }
            _ => Err(Error::Invalid),
        }
    }
}
