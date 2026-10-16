use crate::{load_catalog, CatalogSource, Error, HttpResponse, Result};
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;

#[derive(Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub epoch: u64,
    pub revision: u64,
    pub state: &'static str,
    pub catalog: Option<Value>,
}

impl CatalogSnapshot {
    fn empty(epoch: u64, revision: u64) -> Self {
        Self {
            epoch,
            revision,
            state: "idle",
            catalog: None,
        }
    }
}

/// One owner for display freshness and response ordering. Subscription preview
/// reads publish the same admitted catalog used to validate the displayed offer.
pub struct CatalogRefresh {
    state: Mutex<CatalogSnapshot>,
}

impl Default for CatalogRefresh {
    fn default() -> Self {
        Self {
            state: Mutex::new(CatalogSnapshot::empty(0, 0)),
        }
    }
}

impl CatalogRefresh {
    /// Installing credentials may complete within the same session epoch. Revoke
    /// anonymous display data and its outstanding request before publishing identity.
    pub(crate) fn identity_changed(&self, epoch: u64) {
        let mut state = self.state.lock().expect("catalog state lock");
        if state.epoch <= epoch {
            *state = CatalogSnapshot::empty(epoch, state.revision + 1);
        }
    }

    pub fn snapshot(&self, epoch: u64) -> CatalogSnapshot {
        let state = self.state.lock().expect("catalog state lock");
        if state.epoch == epoch {
            state.clone()
        } else {
            CatalogSnapshot::empty(epoch, state.revision)
        }
    }

    pub async fn refresh(&self, source: &dyn CatalogSource, epoch: u64) -> Result<()> {
        self.fetch(source, epoch).await?.success().map(|_| ())
    }

    pub(crate) async fn fetch(&self, source: &dyn CatalogSource, epoch: u64) -> Result<HttpResponse> {
        source.current(epoch)?;
        let snapshot = {
            let mut state = self.state.lock().expect("catalog state lock");
            if state.epoch > epoch {
                return Err(Error::SessionChanged);
            }
            if state.epoch != epoch {
                *state = CatalogSnapshot::empty(epoch, state.revision);
            }
            state.revision += 1;
            state.state = "loading";
            state.clone()
        };
        let mut pending = Pending {
            owner: self,
            source,
            epoch,
            request: snapshot.revision,
            finished: false,
        };
        source.catalog_changed(&snapshot)?;
        let result = load_catalog(source, epoch).await.and_then(|mut response| {
            if (200..300).contains(&response.status) {
                response.body = crate::catalog_for_channel(response.body, source.channel())?;
            }
            Ok(response)
        });
        source.current(epoch)?;
        let snapshot = self
            .finish(epoch, pending.request, result.as_ref().ok().filter(|response| (200..300).contains(&response.status))
                .map(|response| response.body.clone()))
            .ok_or(Error::SessionChanged)?;
        pending.finished = true;
        source.catalog_changed(&snapshot)?;
        result
    }

    fn finish(&self, epoch: u64, request: u64, catalog: Option<Value>) -> Option<CatalogSnapshot> {
        let mut state = self.state.lock().expect("catalog state lock");
        if state.epoch != epoch || state.revision != request {
            return None;
        }
        state.revision += 1;
        state.state = if catalog.is_some() { "loaded" } else { "error" };
        state.catalog = catalog;
        Some(state.clone())
    }
}

struct Pending<'a> {
    owner: &'a CatalogRefresh,
    source: &'a dyn CatalogSource,
    epoch: u64,
    request: u64,
    finished: bool,
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if !self.finished && self.source.current(self.epoch).is_ok() {
            if let Some(snapshot) = self.owner.finish(self.epoch, self.request, None) {
                // Cancellation cannot leave a reusable price or a permanent loading state.
                // A changed identity never receives the old account's view.
                if self.source.current(self.epoch).is_ok() {
                    let _ = self.source.catalog_changed(&snapshot);
                }
            }
        }
    }
}
