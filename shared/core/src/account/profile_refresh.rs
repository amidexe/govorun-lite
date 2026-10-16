use crate::{load_account_profile, Error, Future, Profile, ProfileSource, Result};
use serde::Serialize;

/// Opaque publication ticket: captured identity and storage/publication revision.
/// Contains no credentials and is shared by refresh, access and checkout effects.
#[derive(Clone, serde::Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileUpdate {
    pub identity: crate::ProfileIdentity,
    pub revision: u64,
}

/// Loss of freshness only revokes new actions. Confirmed server data remains visible.
#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfileUnavailable {
    pub free_access_available: bool,
    pub program_can_activate: bool,
    pub starter_can_activate: bool,
}

impl ProfileUnavailable {
    fn stale() -> Self {
        Self { free_access_available: false, program_can_activate: false, starter_can_activate: false }
    }
}

/// Storage/publication ports share one captured account and revision. An adapter
/// must ignore a failure from an older publication, including after cancellation.
pub trait ProfileRefreshHost: ProfileSource {
    fn begin_profile_refresh(&self, epoch: u64) -> Result<ProfileUpdate>;
    fn publish_refreshed_profile(&self, epoch: u64, update: ProfileUpdate,
        profile: Profile) -> Future<'_, Result<Profile>>;
    fn profile_unavailable(&self, epoch: u64, update: &ProfileUpdate,
        projection: ProfileUnavailable, cause: Error);
}

/// One refresh lifecycle: validated load, durable publication, or stale view.
/// Ancillary devices/sync failures are outside this operation and cannot revoke it.
pub async fn refresh_account_profile(host: &dyn ProfileRefreshHost, epoch: u64) -> Result<Profile> {
    host.current(epoch)?;
    let update = host.begin_profile_refresh(epoch)?;
    let mut pending = PendingProfile { host, epoch, update, completed: false, error: Error::NetworkUnavailable };
    let result = async {
        let profile = load_account_profile(host, epoch, pending.update.identity.clone()).await?;
        host.current(epoch)?;
        let profile = host.publish_refreshed_profile(epoch, pending.update.clone(), profile).await?;
        host.current(epoch)?;
        Ok(profile)
    }.await;
    match &result {
        Ok(_) => pending.completed = true,
        Err(error) => pending.error = *error,
    }
    result
}

struct PendingProfile<'a> {
    host: &'a dyn ProfileRefreshHost,
    epoch: u64,
    update: ProfileUpdate,
    completed: bool,
    error: Error,
}

impl Drop for PendingProfile<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.host.profile_unavailable(self.epoch, &self.update, ProfileUnavailable::stale(), self.error);
        }
    }
}
