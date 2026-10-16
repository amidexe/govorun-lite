//! Private native-client command protocol. Business admission and routing are
//! shared; transports only preserve call order, deliver values and cancel I/O.
use crate::{AccountRequest, AccountSession, Error, Method};
use serde_json::{json, Value};
mod account;
#[path="request_wire.rs"]
pub(crate) mod request_wire;
pub mod auth;
mod lease;
mod sync;
mod workflows;
pub use lease::credit_lease;

pub async fn dispatch(core: &AccountSession, params: Value) -> Result<Value, Error> {
    match params.get("operation").and_then(Value::as_str) {
        Some(
            "syncContentRead"
            | "syncContentWrite"
            | "syncPreferencesRead"
            | "syncPreferencesPatch"
            | "syncStatisticsWrite"
            | "syncStatisticsDelete",
        ) => sync::dispatch(core, params).await,
        Some(
            "accountProfile"
            | "accountDevices"
            | "accountUpdateDevice"
            | "accountRevokeDevice"
            | "accountConsent"
            | "accountDelete",
        ) => account::dispatch(core, params).await,
        Some(
            "activityDelay"
            | "refreshForActivity"
            | "storageRecoveryDelay"
            | "confirmPromotion"
            | "refreshPromotionAccess"
            | "prepareCheckout"
            | "confirmCheckout"
            | "confirmPreparedCheckout"
            | "beginCheckout"
            | "reconcileCheckout"
            | "cancelCheckoutBinding"
            | "accessInteraction"
            | "reconnectRecovery"
            | "styleWindowPresentation"
            | "stylePreparation"
            | "styleReservePlan"
            | "styleUsageLedger"
            | "reserveMaintenance"
            | "creditRecovery"
            | "creditLease"
            | "creditRecoveryWire"
            | "creditRecoveryRequest"
            | "stylePassCrypto"
            | "prepareStyleIssuance"
            | "styleIssuanceJournal"
            | "styleInventory"
            | "styleJobPolling"
            | "styleJobResponse"
            | "styleStartAdmission"
            | "styleTextAdmission"
            | "creditRetirementWire"
            | "styleReserveBinding"
            | "styleReserveTransition"
            | "sameStyleAccessScope",
        ) => workflows::dispatch(core, params).await,
        Some("snapshot") => Ok(core.native_projection()),
        Some("subscriptionAction") => {
            core.subscription_action(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                params["command"].clone(),
            )
            .await
        }
        Some("billingCommand") => {
            let epoch = params["sessionGeneration"]
                .as_u64()
                .ok_or(Error::InvalidRequest)?;
            let command = serde_json::from_value(params["command"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            core.manage_billing(epoch, command, params["confirmed"] == true)
                .await?;
            Ok(core.native_projection())
        }
        Some("refreshBillingStatus") => {
            if params["sessionGeneration"].as_u64() != Some(core.snapshot().session_generation) {
                return Err(Error::SessionChanged);
            }
            core.request(AccountRequest::new(
                Method::Get,
                "/v1/billing/status",
                None,
            )?)
            .await?
            .success()?;
            Ok(core.native_projection())
        }
        Some("refreshCatalog") => {
            core.refresh_catalog(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
            )
            .await?;
            Ok(core.native_projection())
        }
        Some("restore") => {
            core.restore(params["allowInteraction"].as_bool() == Some(true))
                .await?;
            Ok(core.native_projection())
        }
        Some("logout") => {
            core.sign_out().await?;
            Ok(core.native_projection())
        }
        Some("cancelSignIn") => {
            core.cancel_sign_in();
            Ok(core.native_projection())
        }
        Some("acceptOnlineTerms") => {
            let epoch = params["sessionGeneration"]
                .as_u64()
                .ok_or(Error::InvalidRequest)?;
            let version = serde_json::from_value(params["version"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            core.accept_online_terms(epoch, version, params["confirmed"] == true)
                .await?;
            Ok(core.native_projection())
        }
        Some("activateFreeAccess") => {
            let epoch = params["sessionGeneration"]
                .as_u64()
                .ok_or(Error::InvalidRequest)?;
            let command = serde_json::from_value(params["activation"].clone())
                .map_err(|_| Error::InvalidRequest)?;
            core.activate_free_access(epoch, command).await?;
            Ok(core.native_projection())
        }
        Some("http") => http(core, params).await,
        _ => Err(Error::InvalidRequest),
    }
}

async fn http(core: &AccountSession, params: Value) -> Result<Value, Error> {
    let method: Method =
        serde_json::from_value(params["method"].clone()).map_err(|_| Error::InvalidRequest)?;
    let path = params["path"].as_str().ok_or(Error::InvalidRequest)?;
    // PKCE/handoff exchange belongs to AuthFlow. Workers can no longer supply
    // a verifier or bypass callback/state/device-confirmation validation.
    if path == "/v1/auth/session" {
        return Err(Error::InvalidRequest);
    }
    let body = params.get("body").filter(|value| !value.is_null()).cloned();
    let unscoped = matches!((method, path), (Method::Post, "/v1/auth/logout"))
        || (method == Method::Get
            && path == "/v1/billing/catalog"
            && params["publicCatalog"] == true);
    if !unscoped && params["sessionGeneration"].as_u64() != Some(core.snapshot().session_generation)
    {
        return Err(Error::SessionChanged);
    }
    let response = match (method, path) {
        (Method::Post, "/v1/auth/logout") => {
            core.sign_out().await?;
            return Ok(json!({"status": 204, "body": null}));
        }
        (Method::Get, "/v1/account") => {
            let profile = core
                .refresh_for_activity_at(
                    params["sessionGeneration"]
                        .as_u64()
                        .ok_or(Error::InvalidRequest)?,
                    std::time::Duration::ZERO,
                )
                .await?;
            return Ok(json!({"status": 200, "body": profile}));
        }
        (Method::Get, "/v1/devices") | (Method::Put, "/v1/devices/current") => {
            core.device_request(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                AccountRequest::new(method, path, body)?,
            )
            .await?
        }
        (Method::Delete, value) if value.starts_with("/v1/devices/") => {
            core.device_request(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                AccountRequest::new(method, path, body)?,
            )
            .await?
        }
        (Method::Post, "/v1/consents/online-stylization") => {
            let version = body
                .as_ref()
                .and_then(|body| body["version"].as_u64())
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(Error::InvalidRequest)?;
            let epoch = params["sessionGeneration"]
                .as_u64()
                .ok_or(Error::InvalidRequest)?;
            let profile = core
                .accept_online_terms(epoch, version, params["confirmed"] == true)
                .await?;
            return Ok(
                json!({"status": 200, "body": {"onlineConsent": profile.wire()["onlineConsent"]}}),
            );
        }
        (Method::Get, "/v1/billing/catalog") if params["publicCatalog"] == true => {
            core.public_catalog().await?
        }
        (Method::Post, "/v1/billing/tbank/preview") => {
            core.prepare_checkout(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                body.ok_or(Error::InvalidRequest)?,
            )
            .await?
        }
        (Method::Post, "/v1/billing/promo/preview") => {
            let body = body.as_ref().ok_or(Error::InvalidRequest)?;
            let code = body["code"].as_str().ok_or(Error::InvalidRequest)?;
            let offer = match body.get("offerId") {
                None | Some(Value::Null) => None,
                Some(Value::String(value)) => Some(value.as_str()),
                _ => return Err(Error::InvalidRequest),
            };
            core.preview_promotion(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                code,
                offer,
            )
            .await?
        }
        (Method::Post, "/v1/billing/promo/redeem") => {
            let id = body
                .as_ref()
                .and_then(|body| body["previewId"].as_str())
                .ok_or(Error::InvalidRequest)?;
            core.redeem_promotion(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                id,
                params["confirmed"] == true,
            )
            .await?
        }
        _ => {
            let mut request = AccountRequest::new(method, path, body)?;
            if let Some(etag) = params["etag"].as_str() {
                request = request.with_etag(Some(etag.into()))?;
            }
            if let Some(key) = params["idempotencyKey"].as_str() {
                request = request.with_idempotency_key(key.into())?;
            }
            core.request_for_session(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
                request,
            )
            .await?
        }
    };
    if method == Method::Put && path == "/v3/stats/days" && (200..300).contains(&response.status) {
        let _ = core
            .refresh_after_sync(
                params["sessionGeneration"]
                    .as_u64()
                    .ok_or(Error::InvalidRequest)?,
            )
            .await;
    }
    let mut wire = json!({"status": response.status, "body": response.body,
        "etag": response.etag, "retryAfterSeconds": response.retry_after_seconds});
    if method == Method::Post
        && path == "/v1/style-passes/issue"
        && !(200..300).contains(&response.status)
    {
        wire["clientStyleIssueFailure"] = serde_json::to_value(crate::style_issue::failure(
            Some(response.status),
            crate::style_issue::FailureBody::from_value(&wire["body"]),
        )?)
        .map_err(|_| Error::InvalidResponse)?;
    }
    Ok(wire)
}
