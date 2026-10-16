//! Protected account/vault access without a second native account cache.
use super::{Domain, Error, Keychain, Owner};
use serde_json::json;
impl<D: Domain, K: Keychain> Owner<D, K> {
    pub fn account_access(&self) -> &'static str {
        for key in ["account.session", "stylePass.vault"] {
            match self.execute("secret.read", &json!({"key":key,"allowInteraction":false})) {
                Ok(_) => {}
                Err(Error::Denied) => return "permissionRequired",
                Err(Error::Invalid) => return "invalidData",
                Err(Error::Unavailable | Error::Conflict) => return "temporarilyUnavailable",
            }
        }
        "available"
    }
    pub fn recover_vault(&self) -> Result<(), &'static str> {
        self.execute(
            "secret.read",
            &json!({"key":"stylePass.vault","allowInteraction":true}),
        )
        .map(|_| ())
        .map_err(|_| "storage_unavailable")
    }
}
