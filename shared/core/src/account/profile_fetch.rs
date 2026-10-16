use crate::{AccountRequest, AccountSession, Error, Future, HttpResponse, Method, Profile, Result};
use serde::Deserialize;

/// Captured identity facts. A new sign-in may not have received its account ID yet.
#[derive(Default, Clone, serde::Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileIdentity {
    pub account_id: Option<String>,
    pub device_id: Option<String>,
}

pub trait ProfileSource: Send + Sync {
    fn current(&self, epoch: u64) -> Result<()>;
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>>;
}

/// One profile-loading operation for every client. It neither publishes access
/// nor clears a checkout: callers must durably apply the returned profile first.
pub async fn load_account_profile(
    source: &dyn ProfileSource,
    epoch: u64,
    identity: ProfileIdentity,
) -> Result<Profile> {
    for value in [&identity.account_id, &identity.device_id].into_iter().flatten() {
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(Error::InvalidRequest);
        }
    }
    let profile = fetch(source, epoch, Method::Get, "/v1/account").await?;
    require_identity(&profile, identity.account_id.as_deref(), identity.device_id.as_deref())?;
    if !profile.needs_usage_activation() {
        return Ok(profile);
    }
    // Validate identity before the mutation, and bind the mutation reply to the
    // exact account/device from the read even during a first sign-in.
    let activated = fetch(source, epoch, Method::Post, "/v1/style-usage/activate").await?;
    require_identity(&activated, Some(profile.id()), Some(profile.device_id()))?;
    Ok(activated)
}

fn require_identity(profile: &Profile, account: Option<&str>, device: Option<&str>) -> Result<()> {
    if account.is_some_and(|id| id != profile.id()) || device.is_some_and(|id| id != profile.device_id()) {
        return Err(Error::InvalidResponse);
    }
    Ok(())
}

async fn fetch(source: &dyn ProfileSource, epoch: u64, method: Method, path: &str) -> Result<Profile> {
    source.current(epoch)?;
    let result = source.request(epoch, AccountRequest::new(method, path, None)?).await;
    source.current(epoch)?;
    Profile::try_from(result?.success()?.body)
}

impl ProfileSource for AccountSession {
    fn current(&self, epoch: u64) -> Result<()> {
        self.current(epoch)?;
        self.lease().map(|_| ())
    }
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        Box::pin(self.request_at(epoch, request))
    }
}

