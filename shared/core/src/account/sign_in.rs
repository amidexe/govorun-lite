use crate::{
    AccountRequest, Credentials, Error, Future, HttpResponse, Method, Profile, ProfileIdentity,
    ProfileSource, Result,
};
use serde_json::Value;

/// I/O for a provisional sign-in. Credentials may reach protected storage only
/// together with its validated profile. The publication port acknowledges the
/// durable write and fences the captured attempt against cancellation/replacement.
pub trait SignInHost: Send + Sync {
    fn current(&self, epoch: u64) -> Result<()>;
    fn request(
        &self,
        epoch: u64,
        request: AccountRequest,
        access_token: Option<String>,
    ) -> Future<'_, Result<HttpResponse>>;
    fn device_limit(&self, epoch: u64, value: Value) -> Result<()>;
    fn publish(&self, epoch: u64, tokens: Credentials, profile: Profile) -> Future<'_, Result<()>>;
}

pub fn validate_sign_in_request(body: &Value) -> Result<()> {
    if !body.is_object()
        || ["handoff", "codeVerifier", "installationId"]
            .iter()
            .any(|key| {
                body.get(key)
                    .and_then(Value::as_str)
                    .is_none_or(|value| value.is_empty() || value.len() > 4096)
            })
    {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

/// Exchange and profile validation precede publication on every client. The
/// native host never decides whether a partial exchange has established access.
pub async fn complete_sign_in(host: &dyn SignInHost, epoch: u64, body: Value) -> Result<()> {
    validate_sign_in_request(&body)?;
    complete_provisional_sign_in(host, epoch,
        AccountRequest::internal(Method::Post, "/v1/auth/session", Some(body)), true).await
}

pub(crate) fn validate_refresh_import(token: &str) -> Result<()> {
    if !(32..=256).contains(&token.len()) ||
        !token.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) {
        return Err(Error::InvalidRequest);
    }
    Ok(())
}

/// Import is an alternate credential source, with the same provisional profile
/// validation and durable publication as OAuth. Hosts must explicitly enable it.
pub(crate) async fn complete_refresh_import(host: &dyn SignInHost, epoch: u64, token: String) -> Result<()> {
    validate_refresh_import(&token)?;
    complete_provisional_sign_in(host, epoch, AccountRequest::internal(Method::Post,
        "/v1/auth/refresh", Some(serde_json::json!({"refreshToken": token}))), false).await
}

async fn complete_provisional_sign_in(host: &dyn SignInHost, epoch: u64,
    request: AccountRequest, device_replacement: bool) -> Result<()> {
    host.current(epoch)?;
    let response = host
        .request(epoch, request, None)
        .await;
    host.current(epoch)?;
    let response = response?;
    if device_replacement && response.status == 409
        && response.body["error"] == "device_limit_reached"
        && response.body["devices"].is_array()
        && response.body["deviceLimit"].as_u64().is_some()
    {
        host.device_limit(epoch, response.body)?;
        return Err(Error::Http(409));
    }
    let tokens: Credentials =
        serde_json::from_value(response.success()?.body).map_err(|_| Error::InvalidResponse)?;
    tokens.validate().map_err(|_| Error::InvalidResponse)?;
    let source = ProvisionalProfile {
        host,
        access_token: tokens.access_token.clone(),
    };
    let profile = crate::load_account_profile(
        &source,
        epoch,
        ProfileIdentity {
            account_id: None,
            device_id: Some(tokens.device_id.clone()),
        },
    )
    .await?;
    host.current(epoch)?;
    host.publish(epoch, tokens, profile).await?;
    host.current(epoch)
}

struct ProvisionalProfile<'a> {
    host: &'a dyn SignInHost,
    access_token: String,
}

impl ProfileSource for ProvisionalProfile<'_> {
    fn current(&self, epoch: u64) -> Result<()> {
        self.host.current(epoch)
    }
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>> {
        self.host
            .request(epoch, request, Some(self.access_token.clone()))
    }
}
