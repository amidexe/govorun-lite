//! Commit the existing account/reserve decision before acknowledging its
//! view. A failed write leaves the previous view unapplied.
use super::{repository::empty, Additions, Repository};
use crate::{style_inventory::Event, Error, Result};
use serde_json::{json, Value};
impl Repository {
    pub async fn bind(
        &self,
        previous: Option<Value>,
        projection: Value,
        now: u64,
        current: &(dyn Fn() -> bool + Send + Sync),
    ) -> Result<super::InventoryChange> {
        let prior_binding = previous.clone().map(|value| crate::style_reserve::transition(None,
            crate::style_reserve::Projection::decode(value)?)).transpose()?;
        let decision = crate::style_reserve::transition(previous.map(crate::style_reserve::Projection::decode).transpose()?,
            crate::style_reserve::Projection::decode(projection.clone())?)?;
        self.commit(false, current, |old| {
            // Unavailable storage/stale publication preserves recoverable state;
            // it cannot mint a new scope or confirm an empty local reserve.
            let snapshot = &projection["snapshot"];
            if snapshot["storageAvailable"] != true || snapshot["logoutPending"] == true {
                return old.inventory(Additions::default(), Event::Normalize, now, false);
            }
            let mut action = decision["action"]
                .as_str()
                .ok_or(Error::InvalidResponse)?
                .to_owned();
            let mut identity = decision["identityKey"].clone();
            let mut scope = decision["scopeKey"].clone();
            {
                let facts = json!({
                    "previousIdentityKey":old.source["identityKey"].as_str().or_else(||prior_binding.as_ref().and_then(|p|p["identityKey"].as_str())).unwrap_or(""),
                    "previousScopeKey":old.source["scopeKey"].as_str().or_else(||prior_binding.as_ref().and_then(|p|p["scopeKey"].as_str())).unwrap_or(""),
                    "incomingIdentityKey":identity,"incomingScopeKey":scope,
                    "replaceIdentity":action == "clear","invalidateScope":action == "retire"});
                let bound = crate::style_reserve_binding::decide(
                    serde_json::from_value(facts).map_err(|_| Error::InvalidResponse)?,
                );
                action = bound["action"]
                    .as_str()
                    .ok_or(Error::InvalidResponse)?
                    .to_owned();
                identity = bound["identityKey"].clone();
                scope = bound["scopeKey"].clone();
            }
            let base = if action == "clear" {
                empty(old.format())?
            } else {
                old.clone()
            };
            let mut result = base.inventory(
                Additions::default(),
                Event::ChangeScope {
                    compatible: action != "retire",
                },
                now,
                false,
            )?;
            result.candidate.source["identityKey"] = json!(identity.as_str().unwrap_or(""));
            result.candidate.source["scopeKey"] = json!(scope.as_str().unwrap_or(""));
            Ok(result)
        })
        .await
    }
}

/// Persisted account/scope anchors survive logout without authorizing old stored pass state.
pub(super) fn validate_keys(root: &Value) -> Result<()> {
    for name in ["scopeKey", "identityKey"] {
        if root.get(name).is_some_and(|v| v.as_str().is_none_or(|s| s.len() > 65536)) {
            return Err(Error::StorageInvalid);
        }
    }
    Ok(())
}
