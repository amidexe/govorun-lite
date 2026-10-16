use super::{code, offer, PromotionAccess, PromotionEffect};
use crate::{AccountRequest, Error, Future, HttpResponse, Method, Profile, Result};
use chrono::Utc;
use serde_json::{json, Value};

/// Authenticated effects and session facts. Implementations never decide promo benefits.
pub trait PromotionHost: Send + Sync {
    fn terms_version(&self) -> u32;
    fn current(&self, epoch: u64) -> Result<()>;
    fn request(&self, epoch: u64, request: AccountRequest) -> Future<'_, Result<HttpResponse>>;
    fn refresh(&self, epoch: u64) -> Future<'_, Result<Profile>>;
}

#[derive(Default)]
pub struct PromotionService {
    state: tokio::sync::Mutex<Option<Pending>>,
}

struct Pending {
    epoch: u64,
    preview: Value,
    redeemed: Option<Value>,
    access: Option<PromotionAccess>,
}

impl PromotionService {
    /// The shared controller chooses the effect; platform adapters do not
    /// inspect promotion benefits or decide between checkout and redemption.
    pub async fn confirm_promotion(
        &self,
        host: &dyn PromotionHost,
        epoch: u64,
        preview_id: &str,
        confirmed: bool,
    ) -> Result<PromotionEffect> {
        if !confirmed {
            return Err(Error::ConsentUnconfirmed);
        }
        {
            let state = self.state.lock().await;
            host.current(epoch)?;
            let pending = state
                .as_ref()
                .filter(|p| p.epoch == epoch && p.preview["previewId"] == preview_id)
                .ok_or(Error::InvalidRequest)?;
            if pending.redeemed.is_none()
                && offer::timestamp(&pending.preview["previewExpiresAt"])? <= Utc::now()
            {
                return Err(Error::InvalidRequest);
            }
            if pending.preview["offer"]["checkoutRequired"] == true {
                return Ok(PromotionEffect::Checkout {
                    offer_id: pending.preview["offer"]["offerId"]
                        .as_str()
                        .ok_or(Error::InvalidResponse)?
                        .into(),
                    preview_id: preview_id.into(),
                });
            }
        }
        let response = self.redeem_promotion(host, epoch, preview_id, true).await?;
        if !(200..300).contains(&response.status) {
            return Ok(PromotionEffect::Rejected(response));
        }
        let state = self.state.lock().await;
        host.current(epoch)?;
        let access = state
            .as_ref()
            .filter(|p| p.epoch == epoch && p.preview["previewId"] == preview_id)
            .and_then(|p| p.access.clone())
            .ok_or(Error::InvalidResponse)?;
        Ok(PromotionEffect::Redeemed(access))
    }

    /// Shared preview owner for every client. The lock serializes
    /// preview replacement and redemption; account changes still cancel I/O.
    pub async fn preview_promotion(
        &self,
        host: &dyn PromotionHost,
        epoch: u64,
        code: &str,
        requested_offer: Option<&str>,
    ) -> Result<HttpResponse> {
        let mut state = self.state.lock().await;
        host.current(epoch)?;
        *state = None;
        let code = code::normalize(code)?;
        let requested_offer = requested_offer.filter(|value| !value.is_empty());
        if requested_offer.is_some_and(|v| v.len() > 128 || v.chars().any(char::is_control)) {
            return Err(Error::InvalidRequest);
        }
        let response = host
            .request(
                epoch,
                AccountRequest::internal(
                    Method::Post,
                    "/v1/billing/promo/preview",
                    Some(json!({"code": code, "offerId": requested_offer})),
                ),
            )
            .await?;
        if !(200..300).contains(&response.status) {
            return Ok(response);
        }
        if !offer::uuid(&response.body["previewId"])
            || offer::timestamp(&response.body["previewExpiresAt"])? <= Utc::now()
        {
            return Err(Error::InvalidResponse);
        }
        let catalog = if offer::needs_catalog(&response.body["offer"]) {
            Some(
                host.request(
                    epoch,
                    AccountRequest::internal(Method::Get, "/v1/billing/catalog", None),
                )
                .await?
                .success()?
                .body,
            )
        } else {
            None
        };
        if let Some(catalog) = &catalog {
            crate::validate_catalog(catalog)?;
        }
        offer::validate(&response.body["offer"], catalog.as_ref(), requested_offer)?;
        host.current(epoch)?;
        *state = Some(Pending {
            epoch,
            preview: response.body.clone(),
            redeemed: None,
            access: None,
        });
        Ok(response)
    }

    /// Redemption cannot bypass a validated, current preview or create a
    /// subscription. A successful grant is retained before profile enrichment,
    /// so a failed refresh or a double click never grants it a second time.
    pub async fn redeem_promotion(
        &self,
        host: &dyn PromotionHost,
        epoch: u64,
        preview_id: &str,
        confirmed: bool,
    ) -> Result<HttpResponse> {
        if !confirmed || host.terms_version() == 0 {
            return Err(Error::ConsentUnconfirmed);
        }
        let mut state = self.state.lock().await;
        host.current(epoch)?;
        let pending = state
            .as_mut()
            .filter(|p| p.epoch == epoch && p.preview["previewId"] == preview_id)
            .ok_or(Error::InvalidRequest)?;
        if let Some(body) = &pending.redeemed {
            let body = body.clone();
            refresh_access(host, epoch, pending).await?;
            return Ok(success(body));
        }
        if offer::timestamp(&pending.preview["previewExpiresAt"])? <= Utc::now()
            || pending.preview["offer"]["checkoutRequired"] != false
        {
            return Err(Error::InvalidRequest);
        }
        let response = host
            .request(
                epoch,
                AccountRequest::internal(
                    Method::Post,
                    "/v1/billing/promo/redeem",
                    Some(json!({"previewId": preview_id})),
                ),
            )
            .await?;
        if !(200..300).contains(&response.status) {
            if matches!(response.status, 400 | 403 | 404 | 409 | 410) {
                *state = None;
            }
            return Ok(response);
        }
        let redemption = &response.body["redemption"];
        if !offer::uuid(&redemption["redemptionId"])
            || offer::timestamp(&redemption["redeemedAt"])?.timestamp() <= 0
            || redemption["offer"] != pending.preview["offer"]
        {
            return Err(Error::InvalidResponse);
        }
        pending.redeemed = Some(response.body.clone());
        refresh_access(host, epoch, pending).await?;
        Ok(response)
    }

    /// Retries profile publication after a confirmed grant, never redemption.
    pub async fn refresh_redeemed_access(
        &self,
        host: &dyn PromotionHost,
        epoch: u64,
    ) -> Result<PromotionAccess> {
        if host.terms_version() == 0 {
            return Err(Error::InvalidRequest);
        }
        let mut state = self.state.lock().await;
        host.current(epoch)?;
        let pending = state
            .as_mut()
            .filter(|p| p.epoch == epoch && p.redeemed.is_some())
            .ok_or(Error::InvalidRequest)?;
        refresh_access(host, epoch, pending).await?;
        pending.access.clone().ok_or(Error::InvalidResponse)
    }
}

async fn refresh_access(host: &dyn PromotionHost, epoch: u64, pending: &mut Pending) -> Result<()> {
    // Retain the confirmed grant even if this future is cancelled during publication.
    pending.access = Some(PromotionAccess::checking());
    host.current(epoch)?;
    let result = host.refresh(epoch).await;
    host.current(epoch)?;
    if let Ok(profile) = result {
        pending.access = Some(PromotionAccess::confirmed(&profile, host.terms_version()));
    }
    Ok(())
}

fn success(body: Value) -> HttpResponse {
    HttpResponse {
        status: 200,
        body,
        etag: None,
        retry_after_seconds: None,
    }
}
